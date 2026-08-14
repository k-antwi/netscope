use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CveEntry {
    pub id: String,
    pub description: String,
    pub severity: Option<String>,
    pub score: Option<f64>,
    pub published: Option<String>,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CveCheckResult {
    pub query: String,
    pub total_results: u32,
    pub cves: Vec<CveEntry>,
    pub message: Option<String>,
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn empty(query: String, message: &str) -> CveCheckResult {
    CveCheckResult { query, total_results: 0, cves: Vec::new(), message: Some(message.to_string()) }
}

/// Free-text keyword search of the National Vulnerability Database (NVD) for known CVEs
/// mentioning the given product/service name. This is a heuristic lookup, not a precise
/// version match — the caller doesn't have a structured product+version, just a process
/// name/service label, so results are informational and may include unrelated matches.
#[tauri::command]
pub async fn check_cves(query: String) -> Result<CveCheckResult, String> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(empty(query, "No service or process name to search for."));
    }

    let api_key = std::env::var("NVD_API_KEY").ok().filter(|k| !k.is_empty());
    let url = format!(
        "https://services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch={}&resultsPerPage=15",
        url_encode(&query)
    );

    let mut args: Vec<String> = vec!["-s".to_string(), "--max-time".to_string(), "15".to_string()];
    if let Some(key) = &api_key {
        args.push("-H".to_string());
        args.push(format!("apiKey: {}", key));
    }
    args.push(url);

    let output = tokio::task::spawn_blocking(move || Command::new("curl").args(&args).output())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    let body = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();

    if let Some(msg) = json["message"].as_str() {
        return Ok(empty(query, msg));
    }

    let total_results = json["totalResults"].as_u64().unwrap_or(0) as u32;
    let mut cves: Vec<CveEntry> = Vec::new();

    if let Some(vulns) = json["vulnerabilities"].as_array() {
        for v in vulns.iter().take(8) {
            let cve = &v["cve"];
            let id = match cve["id"].as_str() {
                Some(id) if !id.is_empty() => id.to_string(),
                _ => continue,
            };

            let description = cve["descriptions"]
                .as_array()
                .and_then(|arr| arr.iter().find(|d| d["lang"].as_str() == Some("en")))
                .and_then(|d| d["value"].as_str())
                .unwrap_or("")
                .to_string();

            let metrics = &cve["metrics"];
            let (severity, score) = ["cvssMetricV31", "cvssMetricV30", "cvssMetricV2"]
                .iter()
                .find_map(|key| {
                    metrics[key].as_array()?.first().map(|m| {
                        let sev = m["cvssData"]["baseSeverity"]
                            .as_str()
                            .or_else(|| m["baseSeverity"].as_str())
                            .map(|s| s.to_string());
                        let sc = m["cvssData"]["baseScore"].as_f64();
                        (sev, sc)
                    })
                })
                .unwrap_or((None, None));

            let published = cve["published"].as_str().map(|s| s.to_string());

            cves.push(CveEntry {
                url: format!("https://nvd.nist.gov/vuln/detail/{}", id),
                id,
                description,
                severity,
                score,
                published,
            });
        }
    }

    cves.sort_by(|a, b| b.published.cmp(&a.published));

    Ok(CveCheckResult { query, total_results, cves, message: None })
}
