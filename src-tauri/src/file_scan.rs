use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};

/// Event name the frontend listens on for live scan progress.
pub const PROGRESS_EVENT: &str = "file-scan-progress";

const PROGRESS_EVERY_DIRS: u64 = 128;
const PROGRESS_MIN_INTERVAL: Duration = Duration::from_millis(120);
const DEFAULT_LIMIT: usize = 1000;
const MAX_LIMIT: usize = 5000;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileMatch {
    pub name: String,
    pub path: String,
    pub parent: String,
    pub size: u64,
    pub modified: Option<u64>,
    pub is_dir: bool,
    pub exact: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct ScanProgress {
    pub scanned_dirs: u64,
    pub found: usize,
    pub current: String,
}

#[derive(Debug, Serialize)]
pub struct FileScanResult {
    pub query: String,
    pub matches: Vec<FileMatch>,
    pub scanned_dirs: u64,
    pub roots: Vec<String>,
    pub elapsed_ms: u64,
    pub truncated: bool,
    pub cancelled: bool,
    pub denied: u64,
}

/// Shared cancellation flag so a running scan can be stopped from the UI.
#[derive(Default)]
pub struct ScanState {
    cancel: Arc<AtomicBool>,
}

/// Name matcher. Supports `*` / `?` wildcards; otherwise a case-insensitive
/// substring match so partial names ("report.pdf" vs "q3-report.pdf") still hit.
struct Matcher {
    needle: String,
    pattern: Option<Vec<char>>,
}

impl Matcher {
    fn new(query: &str) -> Self {
        let needle = query.to_lowercase();
        let pattern = if needle.contains('*') || needle.contains('?') {
            Some(needle.chars().collect())
        } else {
            None
        };
        Matcher { needle, pattern }
    }

    fn is_match(&self, name_lower: &str) -> bool {
        match &self.pattern {
            Some(p) => glob_match(p, &name_lower.chars().collect::<Vec<char>>()),
            None => name_lower.contains(&self.needle),
        }
    }

    fn is_exact(&self, name_lower: &str) -> bool {
        match &self.pattern {
            Some(_) => false,
            None => name_lower == self.needle,
        }
    }
}

/// Iterative glob matcher with backtracking on `*`.
fn glob_match(pattern: &[char], name: &[char]) -> bool {
    let (mut p, mut n) = (0usize, 0usize);
    let (mut star, mut mark) = (None, 0usize);

    while n < name.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == name[n]) {
            p += 1;
            n += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            mark = n;
            p += 1;
        } else if let Some(s) = star {
            p = s + 1;
            mark += 1;
            n = mark;
        } else {
            return false;
        }
    }

    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

#[cfg(windows)]
fn scan_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for letter in b'A'..=b'Z' {
        let root = PathBuf::from(format!("{}:\\", letter as char));
        if root.is_dir() {
            roots.push(root);
        }
    }
    if roots.is_empty() {
        roots.push(PathBuf::from("C:\\"));
    }
    roots
}

#[cfg(not(windows))]
fn scan_roots() -> Vec<PathBuf> {
    vec![PathBuf::from("/")]
}

/// Pseudo-filesystems, firmlink duplicates and volume snapshots. Walking these
/// either loops forever or reports every file on the machine twice.
#[cfg(not(windows))]
const SKIP_PATHS: &[&str] = &[
    "/proc",
    "/sys",
    "/dev",
    "/run",
    "/net",
    "/.vol",
    "/var/run",
    "/Volumes",
    "/System/Volumes",
    "/private/var/vm",
    "/private/var/db/lockdown",
];

#[cfg(windows)]
const SKIP_PATHS: &[&str] = &[];

const SKIP_DIR_NAMES: &[&str] = &[
    ".Spotlight-V100",
    ".fseventsd",
    ".DocumentRevisions-V100",
    ".TemporaryItems",
    ".Trashes",
    "$RECYCLE.BIN",
    "System Volume Information",
];

fn should_skip(path: &Path, name: &str) -> bool {
    if SKIP_DIR_NAMES.iter().any(|d| d.eq_ignore_ascii_case(name)) {
        return true;
    }
    let as_str = path.to_string_lossy();
    SKIP_PATHS.iter().any(|s| as_str == *s)
}

fn modified_secs(meta: &std::fs::Metadata) -> Option<u64> {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

/// Breadth-first walk of `roots`, collecting entries whose name matches `query`.
/// `on_progress` is called periodically so the caller can report live progress.
fn walk(
    roots: Vec<PathBuf>,
    query: String,
    limit: usize,
    cancel: Arc<AtomicBool>,
    mut on_progress: impl FnMut(ScanProgress),
) -> FileScanResult {
    let started = Instant::now();
    let matcher = Matcher::new(&query);

    let mut queue: VecDeque<PathBuf> = roots.iter().cloned().collect();
    let mut matches: Vec<FileMatch> = Vec::new();
    let mut scanned_dirs: u64 = 0;
    let mut denied: u64 = 0;
    let mut truncated = false;
    let mut last_emit = Instant::now() - PROGRESS_MIN_INTERVAL;

    // Breadth-first, so shallow (usually more relevant) hits surface first.
    while let Some(dir) = queue.pop_front() {
        if truncated || cancel.load(Ordering::Relaxed) {
            break;
        }

        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => {
                denied += 1;
                continue;
            }
        };
        scanned_dirs += 1;

        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let name_lower = name.to_lowercase();

            // file_type() does not follow symlinks, so link loops can't trap us.
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if file_type.is_symlink() {
                continue;
            }
            let is_dir = file_type.is_dir();

            if matcher.is_match(&name_lower) {
                if matches.len() >= limit {
                    // Enough is enough — the UI reports the run as truncated.
                    truncated = true;
                    break;
                }
                let meta = entry.metadata().ok();
                matches.push(FileMatch {
                    name: name.clone(),
                    path: path.to_string_lossy().to_string(),
                    parent: dir.to_string_lossy().to_string(),
                    size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                    modified: meta.as_ref().and_then(modified_secs),
                    is_dir,
                    exact: matcher.is_exact(&name_lower),
                });
            }

            if is_dir && !should_skip(&path, &name) {
                queue.push_back(path);
            }
        }

        if scanned_dirs % PROGRESS_EVERY_DIRS == 0 && last_emit.elapsed() >= PROGRESS_MIN_INTERVAL {
            last_emit = Instant::now();
            on_progress(ScanProgress {
                scanned_dirs,
                found: matches.len(),
                current: dir.to_string_lossy().to_string(),
            });
        }
    }

    // Exact name hits first, then shortest path (closest to a root), then A-Z.
    matches.sort_by(|a, b| {
        b.exact
            .cmp(&a.exact)
            .then_with(|| a.path.matches(std::path::MAIN_SEPARATOR).count().cmp(
                &b.path.matches(std::path::MAIN_SEPARATOR).count(),
            ))
            .then_with(|| a.path.cmp(&b.path))
    });

    FileScanResult {
        query,
        matches,
        scanned_dirs,
        roots: roots.iter().map(|r| r.to_string_lossy().to_string()).collect(),
        elapsed_ms: started.elapsed().as_millis() as u64,
        truncated,
        cancelled: cancel.load(Ordering::Relaxed),
        denied,
    }
}

fn scan_blocking(
    app: AppHandle,
    query: String,
    limit: usize,
    cancel: Arc<AtomicBool>,
) -> FileScanResult {
    walk(scan_roots(), query, limit, cancel, move |p| {
        let _ = app.emit(PROGRESS_EVENT, p);
    })
}

#[tauri::command]
pub async fn scan_files(
    app: AppHandle,
    state: State<'_, ScanState>,
    query: String,
    limit: Option<usize>,
) -> Result<FileScanResult, String> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Err("Enter a file name to search for.".to_string());
    }

    let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let cancel = state.cancel.clone();
    cancel.store(false, Ordering::SeqCst);

    tokio::task::spawn_blocking(move || scan_blocking(app, query, limit, cancel))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cancel_file_scan(state: State<'_, ScanState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glob(pattern: &str, name: &str) -> bool {
        glob_match(
            &pattern.chars().collect::<Vec<char>>(),
            &name.chars().collect::<Vec<char>>(),
        )
    }

    #[test]
    fn glob_handles_wildcards() {
        assert!(glob("*.pdf", "report.pdf"));
        assert!(glob("report.*", "report.pdf"));
        assert!(glob("re*rt.pdf", "report.pdf"));
        assert!(glob("repor?.pdf", "report.pdf"));
        assert!(glob("*", "anything"));
        assert!(!glob("*.pdf", "report.pdfx"));
        assert!(!glob("repor?.pdf", "reporting.pdf"));
    }

    #[test]
    fn matcher_falls_back_to_substring() {
        let m = Matcher::new("Report.PDF");
        assert!(m.is_match("q3-report.pdf"));
        assert!(m.is_exact("report.pdf"));
        assert!(!m.is_exact("q3-report.pdf"));
        assert!(!m.is_match("notes.txt"));
    }

    #[test]
    fn matcher_uses_glob_when_wildcards_present() {
        let m = Matcher::new("*.log");
        assert!(m.is_match("system.log"));
        assert!(!m.is_match("system.log.gz"));
        assert!(!m.is_exact("system.log"));
    }

    #[test]
    fn skips_pseudo_filesystems() {
        assert!(should_skip(Path::new("/anywhere/.Trashes"), ".Trashes"));
        assert!(!should_skip(Path::new("/Users/me/Documents"), "Documents"));
    }

    /// Builds: <tmp>/{notes.txt, a/notes.txt, a/b/notes.txt.bak, a/b/other.txt, .Trashes/notes.txt}
    fn fixture(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("netscope-scan-test-{}", tag));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("a/b")).unwrap();
        std::fs::create_dir_all(root.join(".Trashes")).unwrap();
        std::fs::write(root.join("notes.txt"), b"1").unwrap();
        std::fs::write(root.join("a/notes.txt"), b"22").unwrap();
        std::fs::write(root.join("a/b/notes.txt.bak"), b"333").unwrap();
        std::fs::write(root.join("a/b/other.txt"), b"4").unwrap();
        std::fs::write(root.join(".Trashes/notes.txt"), b"5").unwrap();
        root
    }

    fn run(root: &Path, query: &str, limit: usize) -> FileScanResult {
        walk(
            vec![root.to_path_buf()],
            query.to_string(),
            limit,
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
    }

    #[test]
    fn finds_every_instance_across_subdirectories() {
        let root = fixture("all");
        let result = run(&root, "notes.txt", 100);

        let names: Vec<&str> = result.matches.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names.iter().filter(|n| **n == "notes.txt").count(), 2);
        assert!(names.contains(&"notes.txt.bak"), "substring match expected");
        assert!(!names.contains(&"other.txt"));

        // Exact-name hits sort ahead of partial ones.
        assert!(result.matches[0].exact && result.matches[1].exact);
        assert!(!result.matches[2].exact);

        // Shallowest path first among the exact matches.
        assert_eq!(result.matches[0].path, root.join("notes.txt").to_string_lossy());
        assert_eq!(result.matches[0].size, 1);
        assert!(result.matches[0].modified.is_some());
        assert!(!result.matches[0].is_dir);

        // Skip-listed directories are never descended into.
        assert!(!result.matches.iter().any(|m| m.path.contains(".Trashes")));
        assert!(!result.truncated && !result.cancelled);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn respects_the_result_limit() {
        let root = fixture("limit");
        let result = run(&root, "notes.txt", 2);

        assert_eq!(result.matches.len(), 2);
        assert!(result.truncated);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn stops_when_cancelled() {
        let root = fixture("cancel");
        let cancel = Arc::new(AtomicBool::new(true));
        let result = walk(
            vec![root.clone()],
            "notes.txt".to_string(),
            100,
            cancel,
            |_| {},
        );

        assert!(result.cancelled);
        assert_eq!(result.scanned_dirs, 0);
        assert!(result.matches.is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_query_matches_directories_too() {
        let root = fixture("glob");
        std::fs::create_dir_all(root.join("logs-2026")).unwrap();
        let result = run(&root, "logs-*", 100);

        assert_eq!(result.matches.len(), 1);
        assert!(result.matches[0].is_dir);
        assert_eq!(result.matches[0].name, "logs-2026");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unreadable_directories_are_counted_not_fatal() {
        let root = fixture("denied");
        let result = walk(
            vec![root.join("does-not-exist")],
            "notes.txt".to_string(),
            100,
            Arc::new(AtomicBool::new(false)),
            |_| {},
        );

        assert_eq!(result.denied, 1);
        assert!(result.matches.is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }
}
