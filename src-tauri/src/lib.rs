use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_connections, investigate_ip, get_inbound, investigate_service])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
