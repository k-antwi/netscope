mod cve_check;
mod defender;
mod file_scan;
mod malware_check;

use cve_check::check_cves;
use defender::{cancel_defender_scan, load_last_defender_scan, load_security_reports, neutralize_threat, save_defender_scan, save_security_report, scan_for_threats, DefenderState};
use file_scan::{cancel_file_scan, delete_files, get_file_details, reveal_in_finder, scan_files, ScanState};
use malware_check::check_malware;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::process::Command;
use tauri::Emitter;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InboundConnection {
    pub process: String,
    pub pid: u32,
    pub local_ip: String,
    pub local_port: u16,
    pub remote_ip: String,
    pub remote_port: u16,
    pub state: String,
    pub is_encrypted: bool,
    pub is_localhost_only: bool,
    pub is_all_interfaces: bool,
}

fn parse_addr(addr: &str) -> Option<(String, u16)> {
    if addr.starts_with('[') {
        let b = addr.rfind("]:")?;
        Some((addr[1..b].to_string(), addr[b + 2..].parse().ok()?))
    } else {
        let c = addr.rfind(':')?;
        Some((addr[..c].to_string(), addr[c + 1..].parse().ok()?))
    }
}

fn parse_inbound(output: &str, show_local: bool) -> Vec<InboundConnection> {
    let mut results = Vec::new();
    let mut listen_ports: HashSet<u16> = HashSet::new();
    let encrypted_ports: HashSet<u16> = [443, 465, 587, 993, 8443].iter().cloned().collect();

    // First pass: collect all LISTEN ports for inbound ESTABLISHED detection
    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 { continue; }
        let state = parts[9].trim_matches(|c| c == '(' || c == ')');
        if state != "LISTEN" { continue; }
        let name = parts[8];
        if name.contains("->") { continue; }
        if let Some((_, port)) = parse_addr(name) {
            listen_ports.insert(port);
        }
    }

    // Second pass: emit LISTEN entries + inbound ESTABLISHED
    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 9 { continue; }

        let name = parts[8];
        let state = if parts.len() > 9 {
            parts[9].trim_matches(|c| c == '(' || c == ')')
        } else {
            ""
        };

        let process = parts[0].to_string();
        let pid: u32 = parts[1].parse().unwrap_or(0);

        if state == "LISTEN" && !name.contains("->") {
            if let Some((local_ip, local_port)) = parse_addr(name) {
                let is_localhost_only =
                    local_ip == "127.0.0.1" || local_ip == "::1" || local_ip == "localhost";
                let is_all_interfaces = local_ip == "*";

                if !show_local && is_localhost_only { continue; }

                results.push(InboundConnection {
                    process,
                    pid,
                    local_ip,
                    local_port,
                    remote_ip: String::new(),
                    remote_port: 0,
                    state: "LISTEN".to_string(),
                    is_encrypted: encrypted_ports.contains(&local_port),
                    is_localhost_only,
                    is_all_interfaces,
                });
            }
        } else if state == "ESTABLISHED" && name.contains("->") {
            let arrow = match name.find("->") { Some(p) => p, None => continue };
            let (Some((local_ip, local_port)), Some((remote_ip, remote_port))) = (
                parse_addr(&name[..arrow]),
                parse_addr(&name[arrow + 2..]),
            ) else { continue };

            // Only treat as inbound if local port is a known server port
            if !listen_ports.contains(&local_port) { continue; }

            let is_localhost_only =
                remote_ip == "127.0.0.1" || remote_ip == "::1" || remote_ip.starts_with("fe80:");
            if !show_local && is_localhost_only { continue; }

            results.push(InboundConnection {
                process,
                pid,
                local_ip,
                local_port,
                remote_ip,
                remote_port,
                state: "ESTABLISHED".to_string(),
                is_encrypted: encrypted_ports.contains(&local_port),
                is_localhost_only,
                is_all_interfaces: false,
            });
        }
    }

    results
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteTrace {
    pub ip: String,
    pub rdns: String,
    pub org: String,
    pub country: String,
    pub city: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceInvestigation {
    pub pid: u32,
    pub process_path: String,
    pub local_port: u16,
    pub local_ip: String,
    pub service_name: String,
    pub exposure: String,
    pub active_connections: u32,
    pub is_encrypted: bool,
    pub warnings: Vec<String>,
    pub active_remotes: Vec<RemoteTrace>,
}

const ENCRYPTED_PORTS: &[u16] = &[443, 465, 587, 993, 8443];
const WELL_KNOWN_SERVICES: &[(u16, &str)] = &[
    (21, "FTP"), (22, "SSH"), (23, "Telnet"), (25, "SMTP"), (53, "DNS"),
    (80, "HTTP"), (110, "POP3"), (143, "IMAP"), (443, "HTTPS"),
    (3306, "MySQL"), (5432, "PostgreSQL"), (6379, "Redis"),
    (8080, "HTTP-ALT"), (8443, "HTTPS-ALT"), (27017, "MongoDB"),
];

fn known_service(port: u16) -> String {
    WELL_KNOWN_SERVICES.iter()
        .find(|(p, _)| *p == port)
        .map(|(_, name)| name.to_string())
        .unwrap_or_default()
}

#[tauri::command]
async fn investigate_service(pid: u32, local_port: u16, local_ip: String) -> Result<ServiceInvestigation, String> {
    // Phase 1: process path + full lsof for this port (parallel)
    let (process_path, lsof_raw) = tokio::join!(
        shell_async(format!("ps -p {} -o args= 2>/dev/null | head -1", pid)),
        shell_async(format!("lsof -i :{} -n -P 2>/dev/null", local_port)),
    );

    // Parse unique remote IPs from ESTABLISHED connections (cap at 10)
    let mut seen: HashSet<String> = HashSet::new();
    let mut remote_ips: Vec<String> = Vec::new();
    for line in lsof_raw.lines() {
        if !line.contains("ESTABLISHED") { continue; }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 9 { continue; }
        let name = parts[8];
        if let Some(arrow) = name.find("->") {
            if let Some((ip, _)) = parse_addr(&name[arrow + 2..]) {
                if seen.insert(ip.clone()) && remote_ips.len() < 10 {
                    remote_ips.push(ip);
                }
            }
        }
    }

    let active_connections = remote_ips.len() as u32;

    // Phase 2: trace each remote IP concurrently
    let trace_handles: Vec<_> = remote_ips
        .into_iter()
        .map(|ip| {
            tokio::task::spawn_blocking(move || {
                let rdns = shell(&format!("dig -x {} +short 2>/dev/null | head -1", ip));
                let ipinfo_raw = shell(&format!(
                    "curl -s --max-time 4 https://ipinfo.io/{}/json 2>/dev/null",
                    ip
                ));
                let info: serde_json::Value =
                    serde_json::from_str(&ipinfo_raw).unwrap_or_default();
                RemoteTrace {
                    ip,
                    rdns,
                    org: info["org"].as_str().unwrap_or("").to_string(),
                    country: info["country"].as_str().unwrap_or("").to_string(),
                    city: info["city"].as_str().unwrap_or("").to_string(),
                }
            })
        })
        .collect();

    let mut active_remotes: Vec<RemoteTrace> = Vec::new();
    for handle in trace_handles {
        if let Ok(trace) = handle.await {
            active_remotes.push(trace);
        }
    }

    let is_encrypted = ENCRYPTED_PORTS.contains(&local_port);
    let service_name = known_service(local_port);
    let is_all_interfaces = local_ip == "*" || local_ip == "0.0.0.0" || local_ip == "::";

    let exposure = if is_all_interfaces {
        "Internet-facing (all interfaces)".to_string()
    } else if local_ip == "127.0.0.1" || local_ip == "::1" {
        "Localhost only — not externally reachable".to_string()
    } else {
        format!("Bound to {}", local_ip)
    };

    let mut warnings: Vec<String> = Vec::new();
    if is_all_interfaces && !is_encrypted {
        warnings.push(format!(
            "Port {} is exposed on all interfaces without TLS encryption",
            local_port
        ));
    }
    if local_port == 23 {
        warnings.push("Telnet is unencrypted and should not be exposed".to_string());
    }
    if local_port == 21 {
        warnings.push("FTP transmits credentials in plaintext".to_string());
    }
    if local_port == 80 && is_all_interfaces {
        warnings.push("HTTP is unencrypted — consider redirecting to HTTPS".to_string());
    }

    Ok(ServiceInvestigation {
        pid,
        process_path: process_path.trim().to_string(),
        local_port,
        local_ip,
        service_name,
        exposure,
        active_connections,
        is_encrypted,
        warnings,
        active_remotes,
    })
}

#[tauri::command]
fn get_inbound(show_local: bool) -> Result<Vec<InboundConnection>, String> {
    let output = Command::new("lsof")
        .args(["-i", "-n", "-P"])
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_inbound(&stdout, show_local))
}

const DB_PORTS: &[(u16, &str)] = &[
    (3306, "MySQL"), (5432, "PostgreSQL"), (6379, "Redis"),
    (27017, "MongoDB"), (1433, "SQL Server"), (5984, "CouchDB"),
    (9200, "Elasticsearch"), (9042, "Cassandra"),
];

const STANDARD_OUTBOUND: &[u16] = &[
    21, 22, 25, 53, 80, 110, 143, 443, 465, 587, 993, 995,
    8080, 8443, 123, 5228, 8888, 3000, 4000, 5000,
];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Issue {
    pub severity: String,
    pub category: String,
    pub title: String,
    pub detail: String,
    pub process: String,
    pub pid: u32,
    pub port: Option<u16>,
    pub remote_ip: Option<String>,
}

#[tauri::command]
fn get_issues() -> Result<Vec<Issue>, String> {
    let output = Command::new("lsof")
        .args(["-i", "-n", "-P"])
        .output()
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let inbound = parse_inbound(&stdout, false);
    let outbound = parse_connections(&stdout, false);
    let standard_ports: HashSet<u16> = STANDARD_OUTBOUND.iter().cloned().collect();

    let mut issues: Vec<Issue> = Vec::new();

    // --- Inbound: externally exposed services ---
    for conn in &inbound {
        if conn.state != "LISTEN" || !conn.is_all_interfaces { continue; }

        if let Some((_, db_name)) = DB_PORTS.iter().find(|(p, _)| *p == conn.local_port) {
            issues.push(Issue {
                severity: "critical".to_string(),
                category: "exposed-database".to_string(),
                title: format!("{} database exposed to network", db_name),
                detail: format!(
                    "{} is listening on all interfaces (port {}). Anyone on the network may attempt to connect.",
                    db_name, conn.local_port
                ),
                process: conn.process.clone(), pid: conn.pid,
                port: Some(conn.local_port), remote_ip: None,
            });
            continue;
        }

        if conn.local_port == 23 {
            issues.push(Issue {
                severity: "critical".to_string(),
                category: "plaintext-service".to_string(),
                title: "Telnet service is exposed".to_string(),
                detail: "Telnet transmits all traffic including passwords in plaintext. Disable this service immediately.".to_string(),
                process: conn.process.clone(), pid: conn.pid,
                port: Some(23), remote_ip: None,
            });
            continue;
        }

        if conn.local_port == 21 {
            issues.push(Issue {
                severity: "high".to_string(),
                category: "plaintext-service".to_string(),
                title: "FTP service is exposed".to_string(),
                detail: "FTP sends credentials and data in plaintext. Use SFTP or FTPS instead.".to_string(),
                process: conn.process.clone(), pid: conn.pid,
                port: Some(21), remote_ip: None,
            });
            continue;
        }

        if conn.local_port == 80 {
            issues.push(Issue {
                severity: "warning".to_string(),
                category: "unencrypted-exposure".to_string(),
                title: "Unencrypted HTTP server exposed".to_string(),
                detail: format!(
                    "{} is serving HTTP on all interfaces. Traffic is unencrypted — consider redirecting to HTTPS.",
                    conn.process
                ),
                process: conn.process.clone(), pid: conn.pid,
                port: Some(80), remote_ip: None,
            });
            continue;
        }

        if !conn.is_encrypted {
            issues.push(Issue {
                severity: "warning".to_string(),
                category: "unencrypted-exposure".to_string(),
                title: format!("Port {} exposed without encryption", conn.local_port),
                detail: format!(
                    "{} is listening on all interfaces on port {} without TLS. Traffic to this service is unencrypted.",
                    conn.process, conn.local_port
                ),
                process: conn.process.clone(), pid: conn.pid,
                port: Some(conn.local_port), remote_ip: None,
            });
        }
    }

    // --- Outbound: unusual or plaintext traffic ---
    let mut seen_outbound: HashSet<(String, u16)> = HashSet::new();
    for conn in &outbound {
        if conn.is_local { continue; }
        if !seen_outbound.insert((conn.process.clone(), conn.remote_port)) { continue; }

        if conn.remote_port == 80 {
            issues.push(Issue {
                severity: "info".to_string(),
                category: "plaintext-outbound".to_string(),
                title: format!("{} sending plaintext HTTP", conn.process),
                detail: format!(
                    "{} is connecting to external hosts over unencrypted HTTP (port 80). Traffic may be intercepted.",
                    conn.process
                ),
                process: conn.process.clone(), pid: conn.pid,
                port: Some(80), remote_ip: Some(conn.remote_ip.clone()),
            });
        } else if !standard_ports.contains(&conn.remote_port) {
            issues.push(Issue {
                severity: "info".to_string(),
                category: "unusual-port".to_string(),
                title: format!("{} using non-standard port {}", conn.process, conn.remote_port),
                detail: format!(
                    "{} connected to {}:{} — not a standard well-known service port.",
                    conn.process, conn.remote_ip, conn.remote_port
                ),
                process: conn.process.clone(), pid: conn.pid,
                port: Some(conn.remote_port), remote_ip: Some(conn.remote_ip.clone()),
            });
        }
    }

    let sev_order = |s: &str| match s { "critical" => 0, "high" => 1, "warning" => 2, _ => 3 };
    issues.sort_by_key(|i| sev_order(&i.severity));
    Ok(issues)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BrowserHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BrowserRequest {
    pub id: String,
    pub url: String,
    pub method: String,
    pub status: u16,
    pub status_text: String,
    pub request_headers: Vec<BrowserHeader>,
    pub response_headers: Vec<BrowserHeader>,
    pub request_body: Option<String>,
    pub timing_ms: f64,
    pub from_cache: bool,
    pub initiator: String,
    pub tab_url: String,
    pub timestamp: f64,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_percent: f32,
    pub memory_used_gb: f32,
    pub memory_total_gb: f32,
    pub net_in_bytes: f64,
    pub net_out_bytes: f64,
}

#[tauri::command]
async fn get_system_metrics() -> SystemMetrics {
    // CPU: top -l 2 -s 1 gives two samples 1s apart; shell_async runs it in a blocking thread
    let cpu_out = shell_async("top -l 2 -s 1 -n 0 2>/dev/null".to_string()).await;
    let cpu_percent = cpu_out
        .lines()
        .filter(|l| l.contains("CPU usage"))
        .last()
        .and_then(|l| {
            l.split(',')
                .find(|p| p.contains("idle"))
                .and_then(|p| p.trim().split('%').next())
                .and_then(|s| s.trim().parse::<f32>().ok())
        })
        .map(|idle| (100.0_f32 - idle).max(0.0).min(100.0))
        .unwrap_or(0.0);

    // Memory: vm_stat + sysctl hw.memsize
    let total_bytes: u64 = Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    let vm_out = Command::new("vm_stat")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    let page_size: u64 = vm_out
        .lines()
        .find(|l| l.contains("page size of"))
        .and_then(|l| {
            let s = l.find("page size of ")? + 13;
            l[s..].split_whitespace().next()?.parse().ok()
        })
        .unwrap_or(4096);

    let parse_vm = |key: &str| -> u64 {
        vm_out
            .lines()
            .find(|l| l.trim_start().starts_with(key))
            .and_then(|l| l.splitn(2, ':').nth(1))
            .and_then(|s| s.trim().trim_end_matches('.').parse::<u64>().ok())
            .unwrap_or(0)
    };

    let used_pages = parse_vm("Pages active")
        + parse_vm("Pages wired down")
        + parse_vm("Pages occupied by compressor");

    let memory_used_gb = (used_pages * page_size) as f32 / (1024.0 * 1024.0 * 1024.0);
    let memory_total_gb = total_bytes as f32 / (1024.0 * 1024.0 * 1024.0);

    // Network: netstat -ib cumulative bytes per physical interface
    let net_out_raw = Command::new("netstat")
        .args(["-ib"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    let mut net_in_bytes: f64 = 0.0;
    let mut net_out_bytes: f64 = 0.0;
    for line in net_out_raw.lines().skip(1) {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.len() < 10 { continue; }
        if !p[0].starts_with("en") { continue; }
        if !p[2].starts_with('<') { continue; } // Only <Link#N> rows, avoid per-protocol duplicates
        net_in_bytes += p[6].parse::<f64>().unwrap_or(0.0);
        net_out_bytes += p[9].parse::<f64>().unwrap_or(0.0);
    }

    SystemMetrics { cpu_percent, memory_used_gb, memory_total_gb, net_in_bytes, net_out_bytes }
}

// ── Intruder Detection ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntruderFinding {
    pub id: String,
    pub severity: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub process: String,
    pub pid: u32,
    pub remote_ip: String,
    pub remote_port: u16,
    pub local_port: u16,
    pub recommendation: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IntruderReport {
    pub findings: Vec<IntruderFinding>,
    pub connections_analyzed: usize,
    pub elapsed_ms: u64,
}

const MALICIOUS_PORTS: &[(u16, &str, &str)] = &[
    (1337,  "Hacker / malware signaling port",         "Terminate the process and run a full malware scan"),
    (31337, "Back Orifice classic backdoor port",      "Classic backdoor indicator — terminate immediately and scan"),
    (4444,  "Metasploit default payload listener",     "Known attack-framework port — terminate the process immediately"),
    (5554,  "Sasser worm port",                        "Malware indicator — terminate and run a full system scan"),
    (6667,  "IRC (common botnet C2 channel)",           "IRC is frequently used for botnet command-and-control"),
    (6668,  "IRC (common botnet C2 channel)",           "IRC is frequently used for botnet command-and-control"),
    (6697,  "IRC over TLS (botnet C2)",                "Encrypted IRC — possible hidden botnet C2 channel"),
    (7777,  "Common RAT / backdoor port",              "Terminate immediately and check for persistence mechanisms"),
    (9001,  "Tor relay port",                          "Process may be relaying Tor network traffic"),
    (9050,  "Tor SOCKS proxy port",                    "Process is routing traffic through the Tor anonymization network"),
    (9150,  "Tor Browser SOCKS proxy",                 "Process is routing traffic through Tor"),
    (12345, "NetBus remote-access trojan",             "Classic RAT port — terminate and investigate immediately"),
    (54321, "Common backdoor reverse shell port",      "Known backdoor indicator — investigate immediately"),
    (1080,  "SOCKS proxy (common malware C2 tunnel)",  "Traffic may be tunneled through a proxy to evade detection"),
    (65535, "Max TCP port — often used by backdoors",  "Suspicious port choice commonly used to evade port scans"),
];

const BROWSER_PROCS: &[&str] = &[
    "Google Chrome", "Google Chrome Helper", "firefox", "firefox-bin",
    "Safari", "Chromium", "Brave Browser", "Arc", "Opera", "Vivaldi",
    "Microsoft Edge", "msedge", "chrome",
];

const SUSPICIOUS_DIRS: &[&str] = &[
    "/tmp/", "/private/tmp/", "/var/folders/", "/private/var/folders/",
    "Downloads/", "/.Trash/",
];

fn is_browser_proc(name: &str) -> bool {
    BROWSER_PROCS.iter().any(|b| {
        name.eq_ignore_ascii_case(b) || name.starts_with(*b)
    })
}

fn is_private_ip(ip: &str) -> bool {
    ip == "127.0.0.1"
        || ip == "::1"
        || ip.starts_with("192.168.")
        || ip.starts_with("10.")
        || ip.starts_with("172.")
        || ip.starts_with("fe80:")
}

#[tauri::command]
async fn spot_intruder() -> Result<IntruderReport, String> {
    let start = std::time::Instant::now();

    let (lsof_raw, ps_raw) = tokio::join!(
        shell_async("lsof -i -n -P 2>/dev/null".to_string()),
        shell_async("ps -eo pid=,args= 2>/dev/null".to_string()),
    );

    // Build PID → full process path map from ps
    let mut proc_paths: HashMap<u32, String> = HashMap::new();
    for line in ps_raw.lines() {
        let trimmed = line.trim();
        let mut it = trimmed.splitn(2, ' ');
        if let (Some(pid_s), Some(args)) = (it.next(), it.next()) {
            if let Ok(pid) = pid_s.trim().parse::<u32>() {
                proc_paths.insert(pid, args.trim().to_string());
            }
        }
    }

    let outbound = parse_connections(&lsof_raw, true);
    let inbound = parse_inbound(&lsof_raw, true);
    let connections_analyzed = outbound.len() + inbound.len();

    let mut findings: Vec<IntruderFinding> = Vec::new();
    let mut next_id: u32 = 0;
    macro_rules! new_id { () => {{ next_id += 1; next_id.to_string() }} }

    // ── Rule 1: Outbound connections to known malicious ports ────────────────
    for conn in &outbound {
        if let Some(&(_, label, rec)) = MALICIOUS_PORTS.iter().find(|(p, _, _)| *p == conn.remote_port) {
            findings.push(IntruderFinding {
                id: new_id!(),
                severity: "critical".to_string(),
                category: "known_bad_port".to_string(),
                title: format!("{} connected to malicious port {}", conn.process, conn.remote_port),
                description: format!(
                    "{} (PID {}) has an active connection to {}:{} — {}.",
                    conn.process, conn.pid, conn.remote_ip, conn.remote_port, label
                ),
                process: conn.process.clone(),
                pid: conn.pid,
                remote_ip: conn.remote_ip.clone(),
                remote_port: conn.remote_port,
                local_port: 0,
                recommendation: rec.to_string(),
            });
        }
    }

    // ── Rule 2: Process running from suspicious path making connections ───────
    let mut seen_sus_pid: HashSet<u32> = HashSet::new();
    for conn in &outbound {
        if seen_sus_pid.contains(&conn.pid) { continue; }
        if is_browser_proc(&conn.process) { continue; }
        let path = proc_paths.get(&conn.pid).map(|s| s.as_str()).unwrap_or("");
        if let Some(frag) = SUSPICIOUS_DIRS.iter().find(|&&f| path.contains(f)) {
            seen_sus_pid.insert(conn.pid);
            findings.push(IntruderFinding {
                id: new_id!(),
                severity: "high".to_string(),
                category: "suspicious_process".to_string(),
                title: format!("{} running from suspicious location", conn.process),
                description: format!(
                    "Process {} (PID {}) is making network connections but runs from '{}' — a directory commonly exploited by malware (matched '{}'). Legitimate apps rarely run from here.",
                    conn.process, conn.pid, path, frag
                ),
                process: conn.process.clone(),
                pid: conn.pid,
                remote_ip: conn.remote_ip.clone(),
                remote_port: conn.remote_port,
                local_port: 0,
                recommendation: "Terminate this process, quarantine the binary, and run a full Defender scan.".to_string(),
            });
        }
    }

    // ── Rule 3: Lateral movement — single process hitting multiple local hosts ─
    {
        let mut proc_local: HashMap<(String, u32), HashSet<String>> = HashMap::new();
        for conn in &outbound {
            if is_browser_proc(&conn.process) { continue; }
            if is_private_ip(&conn.remote_ip) && conn.remote_ip != "127.0.0.1" && conn.remote_ip != "::1" {
                proc_local
                    .entry((conn.process.clone(), conn.pid))
                    .or_default()
                    .insert(conn.remote_ip.clone());
            }
        }
        for ((proc, pid), ips) in &proc_local {
            if ips.len() >= 3 {
                findings.push(IntruderFinding {
                    id: new_id!(),
                    severity: "high".to_string(),
                    category: "lateral_movement".to_string(),
                    title: format!("{} contacting {} local hosts", proc, ips.len()),
                    description: format!(
                        "{} (PID {}) has active connections to {} distinct hosts on the local network. This is a hallmark of lateral movement — an attacker pivoting to compromise other machines.",
                        proc, pid, ips.len()
                    ),
                    process: proc.clone(),
                    pid: *pid,
                    remote_ip: ips.iter().next().cloned().unwrap_or_default(),
                    remote_port: 0,
                    local_port: 0,
                    recommendation: "Isolate this machine from the network, terminate the process, and investigate the activity log.".to_string(),
                });
            }
        }
    }

    // ── Rule 4: Port scan / mass external connections ────────────────────────
    {
        let mut proc_ext: HashMap<(String, u32), HashSet<String>> = HashMap::new();
        for conn in &outbound {
            if is_browser_proc(&conn.process) { continue; }
            if !is_private_ip(&conn.remote_ip) {
                proc_ext
                    .entry((conn.process.clone(), conn.pid))
                    .or_default()
                    .insert(conn.remote_ip.clone());
            }
        }
        for ((proc, pid), ips) in &proc_ext {
            if ips.len() >= 10 {
                findings.push(IntruderFinding {
                    id: new_id!(),
                    severity: "high".to_string(),
                    category: "port_scan".to_string(),
                    title: format!("{} connecting to {} external hosts simultaneously", proc, ips.len()),
                    description: format!(
                        "{} (PID {}) has live connections to {} unique external IP addresses. Legitimate desktop apps rarely maintain this many simultaneous external connections — this may indicate port scanning, a botnet, or data exfiltration.",
                        proc, pid, ips.len()
                    ),
                    process: proc.clone(),
                    pid: *pid,
                    remote_ip: ips.iter().next().cloned().unwrap_or_default(),
                    remote_port: 0,
                    local_port: 0,
                    recommendation: "Investigate what this process is doing. If unexpected, terminate it and check for malware.".to_string(),
                });
            }
        }
    }

    // ── Rule 5: Backdoor listener — unusual port exposed on all interfaces ───
    {
        const SAFE_LISTEN: &[u16] = &[
            22, 80, 443, 631, 3000, 3306, 4000, 5000, 5173, 5432, 8080, 8443,
            8000, 8888, 9000, 9229, 27017,
        ];
        for conn in &inbound {
            if conn.state != "LISTEN" || !conn.is_all_interfaces { continue; }
            if SAFE_LISTEN.contains(&conn.local_port) { continue; }

            if let Some(&(_, label, rec)) = MALICIOUS_PORTS.iter().find(|(p, _, _)| *p == conn.local_port) {
                findings.push(IntruderFinding {
                    id: new_id!(),
                    severity: "critical".to_string(),
                    category: "backdoor_listener".to_string(),
                    title: format!("Backdoor port {} open on all interfaces", conn.local_port),
                    description: format!(
                        "{} (PID {}) is listening on port {} on all network interfaces — {}. This may be a remote-access backdoor accepting inbound connections.",
                        conn.process, conn.pid, conn.local_port, label
                    ),
                    process: conn.process.clone(),
                    pid: conn.pid,
                    remote_ip: String::new(),
                    remote_port: 0,
                    local_port: conn.local_port,
                    recommendation: rec.to_string(),
                });
            } else if conn.local_port > 10000 && !is_browser_proc(&conn.process) {
                findings.push(IntruderFinding {
                    id: new_id!(),
                    severity: "medium".to_string(),
                    category: "backdoor_listener".to_string(),
                    title: format!("{} exposing high port {} to the network", conn.process, conn.local_port),
                    description: format!(
                        "{} (PID {}) is listening on port {} on all network interfaces. This non-standard high port is not a recognized service and may be an unauthorized reverse shell or C2 listener.",
                        conn.process, conn.pid, conn.local_port
                    ),
                    process: conn.process.clone(),
                    pid: conn.pid,
                    remote_ip: String::new(),
                    remote_port: 0,
                    local_port: conn.local_port,
                    recommendation: "Confirm you authorized this listener. If not, terminate the process and check for persistence in LaunchAgents.".to_string(),
                });
            }
        }
    }

    // ── Rule 6: Cleartext credential exfiltration (FTP / Telnet outbound) ────
    {
        let mut seen: HashSet<u32> = HashSet::new();
        for conn in &outbound {
            if !seen.insert(conn.pid) { continue; }
            if is_private_ip(&conn.remote_ip) { continue; }
            match conn.remote_port {
                21 => findings.push(IntruderFinding {
                    id: new_id!(),
                    severity: "high".to_string(),
                    category: "cleartext_exfil".to_string(),
                    title: format!("{} transmitting data via cleartext FTP", conn.process),
                    description: format!(
                        "{} (PID {}) is connecting to {} over FTP (port 21). FTP sends credentials and all file data in plaintext — a trivial target for network eavesdroppers.",
                        conn.process, conn.pid, conn.remote_ip
                    ),
                    process: conn.process.clone(),
                    pid: conn.pid,
                    remote_ip: conn.remote_ip.clone(),
                    remote_port: 21,
                    local_port: 0,
                    recommendation: "Replace FTP with SFTP or FTPS. If you did not initiate this transfer, the process may be exfiltrating data.".to_string(),
                }),
                23 => findings.push(IntruderFinding {
                    id: new_id!(),
                    severity: "critical".to_string(),
                    category: "cleartext_exfil".to_string(),
                    title: format!("{} connected over cleartext Telnet", conn.process),
                    description: format!(
                        "{} (PID {}) is connected to {} via Telnet (port 23). Telnet sends everything — including passwords — in plain text, visible to any network observer.",
                        conn.process, conn.pid, conn.remote_ip
                    ),
                    process: conn.process.clone(),
                    pid: conn.pid,
                    remote_ip: conn.remote_ip.clone(),
                    remote_port: 23,
                    local_port: 0,
                    recommendation: "Switch to SSH immediately. If you did not initiate this connection, terminate the process and investigate.".to_string(),
                }),
                _ => {}
            }
        }
    }

    // ── Rule 7: Hidden / dot-prefixed process with network access ────────────
    {
        let mut seen: HashSet<u32> = HashSet::new();
        for conn in &outbound {
            if !seen.insert(conn.pid) { continue; }
            if conn.process.starts_with('.') {
                findings.push(IntruderFinding {
                    id: new_id!(),
                    severity: "high".to_string(),
                    category: "suspicious_process".to_string(),
                    title: format!("Hidden process '{}' making network connections", conn.process),
                    description: format!(
                        "Process '{}' (PID {}) has a dot-prefixed name — a common trick to hide malware from casual inspection. It is actively connecting to {}:{}.",
                        conn.process, conn.pid, conn.remote_ip, conn.remote_port
                    ),
                    process: conn.process.clone(),
                    pid: conn.pid,
                    remote_ip: conn.remote_ip.clone(),
                    remote_port: conn.remote_port,
                    local_port: 0,
                    recommendation: format!("Inspect with: ps -p {} -o args | head -1  — then terminate and quarantine if unauthorized.", conn.pid),
                });
            }
        }
    }

    // Sort critical → high → medium → low
    let sev_rank = |s: &str| match s { "critical" => 0u8, "high" => 1, "medium" => 2, _ => 3 };
    findings.sort_by_key(|f| sev_rank(&f.severity));

    Ok(IntruderReport {
        findings,
        connections_analyzed,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })
}

async fn start_ws_server(app_handle: tauri::AppHandle) {
    let listener = match tokio::net::TcpListener::bind("127.0.0.1:9922").await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[NetScope] WS server failed to bind :9922 — {}", e);
            return;
        }
    };
    eprintln!("[NetScope] Extension bridge listening on ws://127.0.0.1:9922");

    while let Ok((stream, _)) = listener.accept().await {
        let handle = app_handle.clone();
        tokio::spawn(async move {
            let ws = match tokio_tungstenite::accept_async(stream).await {
                Ok(ws) => ws,
                Err(_) => return,
            };
            let _ = handle.emit("extension-connected", ());
            let (_, mut read) = ws.split();
            while let Some(Ok(msg)) = read.next().await {
                if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                    if let Ok(req) = serde_json::from_str::<BrowserRequest>(&text) {
                        let _ = handle.emit("browser-request", &req);
                    }
                }
            }
            let _ = handle.emit("extension-disconnected", ());
        });
    }
}

pub fn run() {
    dotenvy::dotenv().ok();

    tauri::Builder::default()
        .manage(ScanState::default())
        .manage(DefenderState::default())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(start_ws_server(handle));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_connections, investigate_ip, get_inbound, investigate_service, get_issues, get_system_metrics, scan_files, cancel_file_scan, delete_files, get_file_details, reveal_in_finder, check_malware, check_cves, scan_for_threats, cancel_defender_scan, neutralize_threat, save_defender_scan, load_last_defender_scan, save_security_report, load_security_reports, spot_intruder])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
