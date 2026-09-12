use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListeningPort {
    pub protocol: String,       // "tcp" or "udp"
    pub local_ip: String,       // "127.0.0.1", "0.0.0.0", "::1", etc.
    pub port: u16,
    pub process_name: String,   // "agy", "cupsd", "node", etc.
    pub pid: Option<u32>,
    pub is_local_only: bool,    // true if loopback
    pub is_public: bool,        // true if bound to 0.0.0.0, ::, or LAN IP
    pub service_hint: String,   // e.g. "DNS", "HTTP", "Ollama API"
}

impl ListeningPort {
    pub fn get_service_hint(port: u16, proto: &str) -> String {
        match (port, proto) {
            (22, _) => "SSH Server".to_string(),
            (53, _) => "DNS Resolver / systemd-resolved".to_string(),
            (80, "tcp") => "HTTP Web Server".to_string(),
            (443, "tcp") => "HTTPS Web Server".to_string(),
            (631, _) => "IPP / CUPS Print Server".to_string(),
            (1714..=1764, _) => "GSConnect / KDE Connect".to_string(),
            (3000, "tcp") => "Dev Server (React / Vite / Rails)".to_string(),
            (323, "udp") => "NTP Time Sync (Chrony)".to_string(),
            (51413, _) => "BitTorrent Client".to_string(),
            (5353, "udp") => "mDNS / Avahi Zeroconf".to_string(),
            (8000, "tcp") => "Dev Server (Python / Django)".to_string(),
            (8080, "tcp") => "Web Proxy / Alt HTTP".to_string(),
            (11434, "tcp") => "Ollama Local AI API".to_string(),
            (27031..=27040, _) => "Steam In-Home Streaming".to_string(),
            (53317, _) => "LocalSend File Sharing".to_string(),
            _ => format!("{}/{}", port, proto.to_uppercase()),
        }
    }
}

/// Scan all currently listening sockets and return structured information
pub fn scan_listening_ports() -> Vec<ListeningPort> {
    #[cfg(windows)]
    {
        return scan_listening_ports_windows();
    }

    #[cfg(unix)]
    {
        // Attempt fast and comprehensive inspection via `ss -tulpn -H`
        let output = Command::new("ss")
            .args(["-tulpn", "-H"])
            .output();

        let mut results = Vec::new();

        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                results = parse_ss_output(&stdout);
            }
        }

        if results.is_empty() {
            // Fallback: parse /proc/net/tcp and /proc/net/udp
            results = scan_proc_net();
        }

        // Sort: public/exposed ports first, then by port number
        results.sort_by(|a, b| {
            b.is_public
                .cmp(&a.is_public)
                .then_with(|| a.port.cmp(&b.port))
        });

        results
    }
}

#[cfg(windows)]
pub fn scan_listening_ports_windows() -> Vec<ListeningPort> {
    let mut ports = Vec::new();
    if let Ok(output) = Command::new("netstat").args(["-ano", "-p", "tcp"]).output() {
        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            for line in out_str.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 && parts[0].eq_ignore_ascii_case("TCP") && parts[3].eq_ignore_ascii_case("LISTENING") {
                    if let Some((ip, port_str)) = parts[1].rsplit_once(':') {
                        if let Ok(port) = port_str.parse::<u16>() {
                            let pid = parts[4].parse::<u32>().ok();
                            let is_local = is_loopback(ip);
                            let is_pub = !is_local;
                            let service_hint = ListeningPort::get_service_hint(port, "tcp");
                            ports.push(ListeningPort {
                                protocol: "tcp".to_string(),
                                local_ip: ip.to_string(),
                                port,
                                process_name: pid.map(|p| format!("PID {}", p)).unwrap_or_else(|| "Unknown".to_string()),
                                pid,
                                is_local_only: is_local,
                                is_public: is_pub,
                                service_hint,
                            });
                        }
                    }
                }
            }
        }
    }
    ports.sort_by(|a, b| {
        b.is_public
            .cmp(&a.is_public)
            .then_with(|| a.port.cmp(&b.port))
    });
    ports
}

pub fn parse_ss_output(text: &str) -> Vec<ListeningPort> {
    let mut ports = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }

        let proto = parts[0].to_lowercase();
        if proto != "tcp" && proto != "udp" {
            continue;
        }

        let local_addr_full = parts[4];
        let (ip, port) = match parse_ip_and_port(local_addr_full) {
            Some(res) => res,
            None => continue,
        };

        let key = format!("{}:{}:{}", proto, ip, port);
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);

        let (proc_name, pid) = if parts.len() >= 6 {
            parse_process_info(&parts[5..].join(" "))
        } else {
            ("unknown".to_string(), None)
        };

        let is_local_only = is_loopback(&ip);
        let is_public = !is_local_only;
        let service_hint = ListeningPort::get_service_hint(port, &proto);

        ports.push(ListeningPort {
            protocol: proto,
            local_ip: ip,
            port,
            process_name: proc_name,
            pid,
            is_local_only,
            is_public,
            service_hint,
        });
    }

    ports
}

fn parse_ip_and_port(addr: &str) -> Option<(String, u16)> {
    // Examples:
    // "127.0.0.1:41769"
    // "0.0.0.0:5353"
    // "[::1]:631"
    // "[::]:5353"
    // "127.0.0.53%lo:53"
    // "*:80"

    let last_colon = addr.rfind(':')?;
    let ip_part = &addr[..last_colon];
    let port_part = &addr[last_colon + 1..];

    let port: u16 = port_part.parse().ok()?;

    let mut clean_ip = ip_part.to_string();
    if clean_ip.starts_with('[') && clean_ip.ends_with(']') {
        clean_ip = clean_ip[1..clean_ip.len() - 1].to_string();
    }
    if clean_ip == "*" {
        clean_ip = "0.0.0.0".to_string();
    }

    Some((clean_ip, port))
}

fn is_loopback(ip: &str) -> bool {
    ip.starts_with("127.")
        || ip == "::1"
        || ip == "localhost"
        || ip.contains("%lo")
}

fn parse_process_info(users_str: &str) -> (String, Option<u32>) {
    // Example: users:(("chrome",pid=2587,fd=300)) or users:(("agy",pid=4689,fd=13))
    if let Some(start_name) = users_str.find("((\"") {
        let rest = &users_str[start_name + 3..];
        if let Some(end_name) = rest.find('\"') {
            let name = &rest[..end_name];
            let pid = if let Some(pid_idx) = rest.find("pid=") {
                let pid_part = &rest[pid_idx + 4..];
                let num_str: String = pid_part.chars().take_while(|c| c.is_ascii_digit()).collect();
                num_str.parse().ok()
            } else {
                None
            };
            return (name.to_string(), pid);
        }
    }

    // Secondary format: "pid=1234"
    if let Some(pid_idx) = users_str.find("pid=") {
        let pid_part = &users_str[pid_idx + 4..];
        let num_str: String = pid_part.chars().take_while(|c| c.is_ascii_digit()).collect();
        let pid: Option<u32> = num_str.parse().ok();
        return ("process".to_string(), pid);
    }

    ("system/unprivileged".to_string(), None)
}

#[cfg(unix)]
fn scan_proc_net() -> Vec<ListeningPort> {
    let mut ports = Vec::new();
    for (proto, path) in [("tcp", "/proc/net/tcp"), ("udp", "/proc/net/udp")] {
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 4 {
                    continue;
                }
                // Check state: 0A is LISTEN for TCP
                if proto == "tcp" && parts[3] != "0A" {
                    continue;
                }
                if let Some((ip, port)) = parse_proc_hex_addr(parts[1]) {
                    let is_local_only = is_loopback(&ip);
                    let service_hint = ListeningPort::get_service_hint(port, proto);
                    ports.push(ListeningPort {
                        protocol: proto.to_string(),
                        local_ip: ip,
                        port,
                        process_name: "kernel".to_string(),
                        pid: None,
                        is_local_only,
                        is_public: !is_local_only,
                        service_hint,
                    });
                }
            }
        }
    }
    ports
}

#[cfg(unix)]
fn parse_proc_hex_addr(hex_str: &str) -> Option<(String, u16)> {
    let parts: Vec<&str> = hex_str.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let port = u16::from_str_radix(parts[1], 16).ok()?;
    let ip_num = u32::from_str_radix(parts[0], 16).ok()?;
    let b1 = (ip_num & 0xFF) as u8;
    let b2 = ((ip_num >> 8) & 0xFF) as u8;
    let b3 = ((ip_num >> 16) & 0xFF) as u8;
    let b4 = ((ip_num >> 24) & 0xFF) as u8;
    let ip = format!("{}.{}.{}.{}", b1, b2, b3, b4);
    Some((ip, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ss_output() {
        let sample = r#"
tcp    LISTEN    0         4096           127.0.0.1:41769          0.0.0.0:*      users:(("agy",pid=4689,fd=13))
tcp    LISTEN    0         4096             0.0.0.0:8080           0.0.0.0:*      users:(("node",pid=1234,fd=5))
udp    UNCONN    0         0             127.0.0.54:53             0.0.0.0:*      
"#;
        let parsed = parse_ss_output(sample);
        assert_eq!(parsed.len(), 3);

        let agy = parsed.iter().find(|p| p.port == 41769).unwrap();
        assert_eq!(agy.process_name, "agy");
        assert_eq!(agy.pid, Some(4689));
        assert!(agy.is_local_only);
        assert!(!agy.is_public);

        let node = parsed.iter().find(|p| p.port == 8080).unwrap();
        assert_eq!(node.process_name, "node");
        assert_eq!(node.pid, Some(1234));
        assert!(!node.is_local_only);
        assert!(node.is_public);
    }

    #[test]
    fn test_loopback_classification() {
        assert!(is_loopback("127.0.0.1"));
        assert!(is_loopback("127.0.0.53%lo"));
        assert!(is_loopback("::1"));
        assert!(!is_loopback("0.0.0.0"));
        assert!(!is_loopback("::"));
        assert!(!is_loopback("192.168.1.50"));
    }
}
