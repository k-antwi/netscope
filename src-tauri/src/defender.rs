use crate::malware_check;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, State};

// ── Persistence ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredScan {
    pub timestamp: u64,
    pub result: DefenderScanResult,
    pub neutralized: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub id: String,
    pub timestamp: u64,
    pub result: DefenderScanResult,
    pub neutralized: Vec<String>,
}

fn store_path() -> PathBuf {
    home_dir()
        .join(".netscope")
        .join("defender_last_scan.json")
}

fn reports_dir() -> PathBuf {
    home_dir().join(".netscope").join("reports")
}

const REPORT_MAX_AGE_SECS: u64 = 5 * 24 * 60 * 60;

fn prune_old_reports(dir: &PathBuf, now_secs: u64) {
    let cutoff = now_secs.saturating_sub(REPORT_MAX_AGE_SECS);
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if let Ok(ts) = stem.parse::<u64>() {
                if ts < cutoff {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }
}

#[tauri::command]
pub async fn save_security_report(
    result: DefenderScanResult,
    neutralized: Vec<String>,
) -> Result<(), String> {
    let dir = reports_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let id = ts.to_string();
    let report = SecurityReport { id: id.clone(), timestamp: ts, result, neutralized };
    let json = serde_json::to_string(&report).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(format!("{}.json", id)), json).map_err(|e| e.to_string())?;

    prune_old_reports(&dir, ts);
    Ok(())
}

#[tauri::command]
pub async fn load_security_reports() -> Result<Vec<SecurityReport>, String> {
    let dir = reports_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let cutoff = now.saturating_sub(REPORT_MAX_AGE_SECS);

    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    let mut reports: Vec<SecurityReport> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let ts: u64 = match stem.parse() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if ts < cutoff {
            let _ = std::fs::remove_file(&path);
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(report) = serde_json::from_str::<SecurityReport>(&content) {
                reports.push(report);
            }
        }
    }

    reports.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(reports)
}

#[tauri::command]
pub async fn save_defender_scan(
    result: DefenderScanResult,
    neutralized: Vec<String>,
) -> Result<(), String> {
    let dir = home_dir().join(".netscope");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let stored = StoredScan { timestamp: ts, result, neutralized };
    let json = serde_json::to_string(&stored).map_err(|e| e.to_string())?;
    std::fs::write(store_path(), json).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_last_defender_scan() -> Result<Option<StoredScan>, String> {
    let path = store_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let stored: StoredScan = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(Some(stored))
}

#[derive(Debug, Default)]
pub struct DefenderState {
    pub cancel: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatItem {
    pub path: String,
    pub name: String,
    pub threat_type: String,
    pub severity: String,
    pub reason: String,
    pub size: u64,
    pub modified: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenderProgress {
    pub scanned_files: u64,
    pub threats_found: u32,
    pub current: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenderScanResult {
    pub threats: Vec<ThreatItem>,
    pub scanned_files: u64,
    pub elapsed_ms: u64,
    pub scan_type: String,
    pub cancelled: bool,
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

fn modified_secs(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

/// Checks whether a suspicious path appears as a plist string value (inside <string>…</string>).
/// This avoids false positives from paths mentioned in comments or other non-executable contexts.
fn plist_string_contains(content: &str, path_prefix: &str) -> bool {
    let tag = "<string>";
    let mut pos = 0;
    while let Some(rel) = content[pos..].find(tag) {
        let after_tag = pos + rel + tag.len();
        if content[after_tag..].starts_with(path_prefix) {
            return true;
        }
        pos = after_tag;
    }
    false
}

fn analyze_file(path: &Path, home: &Path) -> Option<ThreatItem> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.is_dir() {
        return None;
    }

    let name = path.file_name()?.to_string_lossy().to_string();
    let path_str = path.to_string_lossy().to_string();
    let size = meta.len();
    let modified = modified_secs(path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // LaunchAgent / LaunchDaemon plist checks
    let launch_dirs: &[PathBuf] = &[
        home.join("Library").join("LaunchAgents"),
        PathBuf::from("/Library/LaunchAgents"),
        PathBuf::from("/Library/LaunchDaemons"),
    ];
    let in_launch_dir = launch_dirs
        .iter()
        .any(|d| path.parent().map(|p| p == d.as_path()).unwrap_or(false));

    if in_launch_dir && ext == "plist" {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        // Only flag if a suspicious path appears as a <string> value — avoids flagging
        // plists that merely mention these paths in comments or log-dir keys.
        let suspicious_prefixes: &[(&str, &str)] = &[
            ("/tmp/", "/tmp"),
            ("/var/folders/", "/var/folders"),
            ("/Users/Shared/", "/Users/Shared"),
            ("/.", "hidden path"),
        ];
        for &(prefix, label) in suspicious_prefixes {
            if plist_string_contains(&content, prefix) {
                return Some(ThreatItem {
                    path: path_str,
                    name,
                    threat_type: "suspicious_launchagent".to_string(),
                    severity: "high".to_string(),
                    reason: format!(
                        "LaunchAgent plist executes a program from a suspicious location (\"{}\")",
                        label
                    ),
                    size,
                    modified,
                });
            }
        }
        // Also check if it references the user's Downloads folder
        let downloads_str = home.join("Downloads").to_string_lossy().to_string() + "/";
        if plist_string_contains(&content, &downloads_str) {
            return Some(ThreatItem {
                path: path_str,
                name,
                threat_type: "suspicious_launchagent".to_string(),
                severity: "high".to_string(),
                reason: "LaunchAgent plist executes a program from the Downloads folder"
                    .to_string(),
                size,
                modified,
            });
        }
        return None;
    }

    // Downloads-specific checks
    let downloads = home.join("Downloads");
    if path.starts_with(&downloads) {
        let script_exts = ["sh", "bash", "zsh", "py", "rb", "pl", "command", "tool"];
        if script_exts.contains(&ext.as_str()) && is_executable(path) {
            return Some(ThreatItem {
                path: path_str,
                name,
                threat_type: "suspicious_script".to_string(),
                severity: "medium".to_string(),
                reason: format!(
                    "Executable {} script found in Downloads folder",
                    ext.to_uppercase()
                ),
                size,
                modified,
            });
        }
        if ext.is_empty() && is_executable(path) && size > 4096 {
            return Some(ThreatItem {
                path: path_str,
                name,
                threat_type: "suspicious_executable".to_string(),
                severity: "high".to_string(),
                reason: "Executable binary with no file extension found in Downloads folder"
                    .to_string(),
                size,
                modified,
            });
        }
        return None;
    }

    // Hidden executable checks
    if name.starts_with('.') && is_executable(path) && size > 1024 {
        let known_safe = [
            ".bash_history",
            ".zsh_history",
            ".profile",
            ".bashrc",
            ".zshrc",
            ".gitconfig",
            ".npmrc",
            ".yarnrc",
        ];
        if !known_safe.contains(&name.as_str()) {
            return Some(ThreatItem {
                path: path_str,
                name,
                threat_type: "hidden_executable".to_string(),
                severity: "medium".to_string(),
                reason: "Hidden file with executable permissions — may be concealing malicious code"
                    .to_string(),
                size,
                modified,
            });
        }
    }

    None
}

/// Calls the malware database (VirusTotal + MalwareBazaar) synchronously from a blocking context.
/// Returns the check result, which includes status "clean", "malicious", "suspicious",
/// "unknown" (not in DB), "no_api_key", or "error".
fn db_verify(path: &Path) -> malware_check::MalwareCheckResult {
    let path_str = path.to_string_lossy().to_string();
    tokio::runtime::Handle::current()
        .block_on(malware_check::check_file_hash(&path_str))
}

fn collect_scan_dirs(scan_type: &str, custom_paths: &[String]) -> Vec<PathBuf> {
    let home = home_dir();
    let dirs: Vec<PathBuf> = match scan_type {
        "quick" => vec![
            home.join("Downloads"),
            home.join("Library").join("LaunchAgents"),
            PathBuf::from("/Library/LaunchAgents"),
            PathBuf::from("/Library/LaunchDaemons"),
            PathBuf::from("/tmp"),
        ],
        "full" => vec![
            home.clone(),
            PathBuf::from("/Library/LaunchAgents"),
            PathBuf::from("/Library/LaunchDaemons"),
            PathBuf::from("/tmp"),
        ],
        "custom" => custom_paths.iter().map(PathBuf::from).collect(),
        _ => vec![],
    };
    dirs.into_iter().filter(|d| d.exists()).collect()
}

fn run_scan(
    scan_type: String,
    custom_paths: Vec<String>,
    cancel: Arc<AtomicBool>,
    handle: tauri::AppHandle,
) -> DefenderScanResult {
    let start = std::time::Instant::now();
    let home = home_dir();
    let dirs = collect_scan_dirs(&scan_type, &custom_paths);

    let mut scanned: u64 = 0;
    let mut threats: Vec<ThreatItem> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<PathBuf> = VecDeque::new();

    for dir in dirs {
        queue.push_back(dir);
    }

    let skip_names: &[&str] = &[
        ".git",
        "node_modules",
        "Caches",
        "Logs",
        "Mail",
        ".Trash",
        "CoreData",
        "CloudDocs",
        "com.apple.mail",
    ];

    while let Some(dir) = queue.pop_front() {
        if cancel.load(Ordering::Relaxed) {
            return DefenderScanResult {
                threats,
                scanned_files: scanned,
                elapsed_ms: start.elapsed().as_millis() as u64,
                scan_type,
                cancelled: true,
            };
        }

        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            if cancel.load(Ordering::Relaxed) {
                break;
            }

            let path = entry.path();
            let path_str = path.to_string_lossy().to_string();

            if seen.contains(&path_str) {
                continue;
            }
            seen.insert(path_str.clone());

            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            // Don't follow symlinks
            if file_type.is_symlink() {
                continue;
            }

            if file_type.is_dir() {
                let dir_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if skip_names.iter().any(|s| dir_name == *s) {
                    continue;
                }
                // For quick scan only recurse into Downloads subdirectories
                if scan_type == "quick" {
                    let home_downloads = home.join("Downloads");
                    if path.starts_with(&home_downloads) {
                        queue.push_back(path);
                    }
                } else {
                    queue.push_back(path);
                }
                continue;
            }

            if !file_type.is_file() {
                continue;
            }

            scanned += 1;

            if scanned % 200 == 0 {
                let _ = handle.emit(
                    "defender-progress",
                    DefenderProgress {
                        scanned_files: scanned,
                        threats_found: threats.len() as u32,
                        current: path_str,
                    },
                );
            }

            if let Some(mut threat) = analyze_file(&path, &home) {
                // LaunchAgent plist threats are content-based (not binary) — no hash to verify.
                // All other heuristic candidates are verified against VirusTotal + MalwareBazaar
                // before being reported, to eliminate false positives.
                let confirmed = if threat.threat_type == "suspicious_launchagent" {
                    true
                } else {
                    let check = db_verify(&path);
                    match check.status.as_str() {
                        "malicious" => {
                            let family = check
                                .malware_bazaar
                                .as_ref()
                                .and_then(|b| b.signature.as_deref())
                                .map(|s| format!(" ({})", s))
                                .unwrap_or_default();
                            threat.severity = "critical".to_string();
                            threat.reason = format!(
                                "{} — confirmed malicious{} by {}/{} antivirus engines",
                                threat.reason,
                                family,
                                check.malicious,
                                check.total_engines
                            );
                            true
                        }
                        "suspicious" => {
                            threat.reason = format!(
                                "{} — flagged suspicious by {}/{} antivirus engines",
                                threat.reason, check.suspicious, check.total_engines
                            );
                            true
                        }
                        // Database confirms the file is clean — discard the heuristic match
                        "clean" => false,
                        // Not in any database — absence of a record is not confirmation of malice
                        "unknown" => false,
                        // Cannot verify without an API key — suppress rather than false-positive
                        "no_api_key" => false,
                        // Network error or unexpected response — cannot confirm, suppress
                        _ => false,
                    }
                };

                if confirmed {
                    threats.push(threat);
                }
            }
        }
    }

    let _ = handle.emit(
        "defender-progress",
        DefenderProgress {
            scanned_files: scanned,
            threats_found: threats.len() as u32,
            current: String::new(),
        },
    );

    DefenderScanResult {
        threats,
        scanned_files: scanned,
        elapsed_ms: start.elapsed().as_millis() as u64,
        scan_type,
        cancelled: false,
    }
}

#[tauri::command]
pub async fn scan_for_threats(
    scan_type: String,
    custom_paths: Vec<String>,
    state: State<'_, DefenderState>,
    app: tauri::AppHandle,
) -> Result<DefenderScanResult, String> {
    state.cancel.store(false, Ordering::Relaxed);
    let cancel = state.cancel.clone();

    tokio::task::spawn_blocking(move || run_scan(scan_type, custom_paths, cancel, app))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel_defender_scan(state: State<'_, DefenderState>) -> Result<(), String> {
    state.cancel.store(true, Ordering::Relaxed);
    Ok(())
}

/// Returns true if the app has been granted Full Disk Access in
/// System Settings → Privacy & Security → Full Disk Access.
/// The TCC database itself is only readable with FDA, so a successful
/// metadata read is a reliable proxy.
#[tauri::command]
pub fn check_full_disk_access() -> bool {
    std::fs::metadata("/Library/Application Support/com.apple.TCC/TCC.db").is_ok()
}

/// Opens System Settings at the Full Disk Access pane.
#[tauri::command]
pub fn open_full_disk_access_settings() -> Result<(), String> {
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Pre-touches the home directory and every macOS-protected subfolder so that
/// all TCC permission dialogs appear before the scan starts rather than
/// interrupting it mid-run.
#[tauri::command]
pub async fn request_scan_permissions() -> Result<(), String> {
    let home = home_dir();
    let dirs = [
        home.clone(),
        home.join("Downloads"),
        home.join("Documents"),
        home.join("Desktop"),
        home.join("Library").join("LaunchAgents"),
        PathBuf::from("/Library/LaunchAgents"),
        PathBuf::from("/Library/LaunchDaemons"),
    ];
    tokio::task::spawn_blocking(move || {
        for dir in &dirs {
            if dir.exists() {
                let _ = std::fs::read_dir(dir);
            }
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn neutralize_threat(path: String) -> Result<String, String> {
    let src = PathBuf::from(&path);
    if !src.exists() {
        return Err("File not found — it may have already been removed.".to_string());
    }

    let home = home_dir();
    let quarantine = home.join(".netscope").join("quarantine");
    std::fs::create_dir_all(&quarantine).map_err(|e| e.to_string())?;

    let file_name = src
        .file_name()
        .ok_or("Invalid file name")?
        .to_string_lossy()
        .to_string();

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let dest = quarantine.join(format!("{}_{}", ts, file_name));
    std::fs::rename(&src, &dest).map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().to_string())
}
