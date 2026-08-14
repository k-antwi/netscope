mod file_scan;

use file_scan::{cancel_file_scan, scan_files, ScanState};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Connection {
    pub process: String,
    pub pid: u32,
    pub local_addr: String,
    pub remote_ip: String,
    pub remote_port: u16,
    pub state: String,
    pub is_https: bool,
    pub is_local: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpInvestigation {
    pub ip: String,
    pub rdns: String,
    pub asn_org: String,
    pub country: String,
    pub city: String,
    pub tls_subject: String,
    pub tls_issuer: String,
    pub whois_netname: String,
    pub whois_org: String,
    pub suspicious: bool,
    pub suspicious_reasons: Vec<String>,
}

fn parse_connections(output: &str, show_local: bool) -> Vec<Connection> {
    let mut connections = Vec::new();

    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 9 {
            continue;
        }

        let name = parts[8];
        if !name.contains("->") {
            continue;
        }

        let state = if parts.len() > 9 {
            parts[9].trim_matches(|c| c == '(' || c == ')')
        } else {
            ""
        };

        if state != "ESTABLISHED" && state != "SYN_SENT" {
            continue;
        }

        let arrow = match name.find("->") {
            Some(p) => p,
            None => continue,
        };

        let local = &name[..arrow];
        let remote = &name[arrow + 2..];

        let (remote_ip, remote_port) = if remote.starts_with('[') {
            match remote.rfind("]:") {
                Some(b) => {
                    let ip = remote[1..b].to_string();
                    let port: u16 = remote[b + 2..].parse().unwrap_or(0);
                    (ip, port)
                }
                None => continue,
            }
        } else {
            match remote.rfind(':') {
                Some(c) => {
                    let ip = remote[..c].to_string();
                    let port: u16 = remote[c + 1..].parse().unwrap_or(0);
                    (ip, port)
                }
                None => continue,
            }
        };

        let is_local = remote_ip == "127.0.0.1"
            || remote_ip == "::1"
            || remote_ip.starts_with("192.168.")
            || remote_ip.starts_with("10.")
            || remote_ip.starts_with("172.")
            || remote_ip.starts_with("fe80:");

        if !show_local && is_local {
            continue;
        }

        connections.push(Connection {
            process: parts[0].to_string(),
            pid: parts[1].parse().unwrap_or(0),
            local_addr: local.to_string(),
            remote_ip,
            remote_port,
            state: state.to_string(),
            is_https: remote_port == 443,
            is_local,
        });
    }

    connections
}

#[tauri::command]
fn get_connections(show_local: bool) -> Result<Vec<Connection>, String> {
    let output = Command::new("lsof")
        .args(["-i", "-n", "-P"])
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_connections(&stdout, show_local))
}

fn shell(cmd: &str) -> String {
    Command::new("sh")
        .args(["-c", cmd])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

async fn shell_async(cmd: String) -> String {
    tokio::task::spawn_blocking(move || shell(&cmd))
        .await
        .unwrap_or_default()
}

#[tauri::command]
async fn investigate_ip(ip: String, port: u16) -> Result<IpInvestigation, String> {
    let tls_port = if port == 443 || port == 8443 { port } else { 443 };

    let (rdns, whois_raw, ipinfo_raw, tls_raw) = tokio::join!(
        shell_async(format!("dig -x {} +short 2>/dev/null | head -1", ip)),
        shell_async(format!("whois {} 2>/dev/null", ip)),
        shell_async(format!(
            "curl -s --max-time 5 https://ipinfo.io/{}/json 2>/dev/null",
            ip
        )),
        shell_async(format!(
            "echo | openssl s_client -connect {}:{} -servername {} 2>/dev/null \
             | openssl x509 -noout -subject -issuer 2>/dev/null",
            ip, tls_port, ip
        )),
    );

    // Parse WHOIS
    let mut whois_netname = String::new();
    let mut whois_org = String::new();
    for line in whois_raw.lines() {
        let lower = line.to_lowercase();
        if whois_netname.is_empty() && lower.starts_with("netname:") {
            whois_netname = line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
        }
        if whois_org.is_empty()
            && (lower.starts_with("orgname:")
                || lower.starts_with("org-name:")
                || lower.starts_with("organization:"))
        {
            whois_org = line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
        }
    }

    // Parse ipinfo
    let ipinfo: serde_json::Value = serde_json::from_str(&ipinfo_raw).unwrap_or_default();
    let asn_org = ipinfo["org"].as_str().unwrap_or("").to_string();
    let country = ipinfo["country"].as_str().unwrap_or("").to_string();
    let city = ipinfo["city"].as_str().unwrap_or("").to_string();

    // Parse TLS cert
    let mut tls_subject = String::new();
    let mut tls_issuer = String::new();
    for line in tls_raw.lines() {
        if line.starts_with("subject=") {
            tls_subject = line["subject=".len()..].trim().to_string();
        } else if line.starts_with("issuer=") {
            tls_issuer = line["issuer=".len()..].trim().to_string();
        }
    }

    // Suspicion analysis
    let mut suspicious_reasons: Vec<String> = Vec::new();
    let org_lower = asn_org.to_lowercase();
    let whois_lower = whois_org.to_lowercase();

    let bare_vps = ["digitalocean", "vultr", "linode", "hetzner", "ovh"];
    for provider in &bare_vps {
        if org_lower.contains(provider) || whois_lower.contains(provider) {
            suspicious_reasons.push(format!("Hosted on {} (bare cloud VPS)", provider));
            break;
        }
    }

    if rdns.is_empty() {
        suspicious_reasons.push("No reverse DNS record".to_string());
    }

    if !tls_subject.is_empty() {
        let tls_lower = tls_subject.to_lowercase();
        let trusted = ["microsoft", "google", "amazon", "apple", "cloudflare", "fastly", "akamai", "zoom"];
        if !trusted.iter().any(|t| tls_lower.contains(t))
            && (org_lower.contains("digitalocean")
                || org_lower.contains("vultr")
                || org_lower.contains("linode"))
        {
            suspicious_reasons.push(format!(
                "TLS cert ({}) does not match expected organization",
                tls_subject
            ));
        }
    }

    Ok(IpInvestigation {
        ip,
        rdns,
        asn_org,
        country,
        city,
        tls_subject,
        tls_issuer,
        whois_netname,
        whois_org,
        suspicious: !suspicious_reasons.is_empty(),
        suspicious_reasons,
    })
}

pub fn run() {
    tauri::Builder::default()
        .manage(ScanState::default())
        .invoke_handler(tauri::generate_handler![
            get_connections,
            investigate_ip,
            scan_files,
            cancel_file_scan
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
