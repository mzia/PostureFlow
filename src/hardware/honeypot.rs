use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub const HONEYPOT_INCIDENTS_PATH: &str = "/run/postureflow_honeypot_incidents.json";
pub const HONEYPOT_FALLBACK_PATH: &str = "/tmp/postureflow_honeypot_incidents.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HoneypotIncident {
    pub timestamp: u64,
    pub source_ip: String,
    pub trap_port: u16,
    pub blocked: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HoneypotConfig {
    #[serde(default = "default_false")]
    pub enabled: bool,
    #[serde(default = "default_trap_ports")]
    pub trap_ports: Vec<u16>,
    #[serde(default = "default_true")]
    pub auto_block_offender: bool,
    #[serde(default = "default_true")]
    pub notify_on_intrusion: bool,
}

fn default_false() -> bool {
    false
}

fn default_true() -> bool {
    true
}

fn default_trap_ports() -> Vec<u16> {
    vec![2222, 8080, 4450]
}

impl Default for HoneypotConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            trap_ports: default_trap_ports(),
            auto_block_offender: true,
            notify_on_intrusion: true,
        }
    }
}

pub fn get_incidents_file_path() -> &'static Path {
    if Path::new("/run").exists() {
        Path::new(HONEYPOT_INCIDENTS_PATH)
    } else {
        Path::new(HONEYPOT_FALLBACK_PATH)
    }
}

/// Load recent honeypot intrusion incidents
pub fn load_recent_incidents() -> Vec<HoneypotIncident> {
    let path = get_incidents_file_path();
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(list) = serde_json::from_str::<Vec<HoneypotIncident>>(&data) {
            return list;
        }
    }
    Vec::new()
}

/// Record a detected intrusion incident
pub fn record_incident(incident: HoneypotIncident) {
    let mut incidents = load_recent_incidents();
    incidents.push(incident);

    // Keep the most recent 100 incidents
    if incidents.len() > 100 {
        let excess = incidents.len() - 100;
        incidents.drain(0..excess);
    }

    let path = get_incidents_file_path();
    if let Ok(json) = serde_json::to_string_pretty(&incidents) {
        let _ = fs::write(path, json);
    }
}

/// Start an async TCP honeypot trap listener on a specific port
pub async fn run_port_trap_listener(
    port: u16,
    auto_block: bool,
    notify: bool,
    shutdown: Arc<AtomicBool>,
) {
    let bind_addr = format!("0.0.0.0:{}", port);
    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(l) => {
            println!("[+] 🪤 Decoy Honeypot Trap listening on TCP port {}", port);
            l
        }
        Err(e) => {
            eprintln!("[-] Could not bind honeypot trap to {}: {}", bind_addr, e);
            return;
        }
    };

    while !shutdown.load(Ordering::Relaxed) {
        tokio::select! {
            accept_result = listener.accept() => {
                if let Ok((_stream, peer_addr)) = accept_result {
                    let peer_ip = peer_addr.ip().to_string();
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);

                    // Ignore loopback tests
                    if peer_ip == "127.0.0.1" || peer_ip == "::1" {
                        continue;
                    }

                    println!(
                        "[!] ⚠️ HONEYPOT INTRUSION: Rogue scan detected from {} on decoy port {}",
                        peer_ip, port
                    );

                    let mut blocked = false;
                    if auto_block {
                        if let Err(e) = crate::system::block_offender_ip(&peer_ip) {
                            eprintln!("[-] Failed to block offender IP {}: {}", peer_ip, e);
                        } else {
                            println!("[✔] Automatically blocked offender IP {} via firewall", peer_ip);
                            blocked = true;
                        }
                    }

                    if notify {
                        let _ = crate::system::execute(
                            "notify-send",
                            &[
                                "-u",
                                "critical",
                                "PostureFlow Honeypot Trap",
                                &format!(
                                    "⚠️ Port scan intrusion detected from IP {}\nTargeting trap port {}\nAction: {}",
                                    peer_ip,
                                    port,
                                    if blocked { "Firewall Blocked" } else { "Logged" }
                                ),
                            ],
                        );
                    }

                    let incident = HoneypotIncident {
                        timestamp: now,
                        source_ip: peer_ip,
                        trap_port: port,
                        blocked,
                        description: format!("Connection attempt to decoy honeypot trap port {}", port),
                    };

                    record_incident(incident);
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) => {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }
            }
        }
    }

    println!("[*] Honeypot listener on port {} shut down cleanly", port);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_honeypot_default_config() {
        let cfg = HoneypotConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.trap_ports, vec![2222, 8080, 4450]);
        assert!(cfg.auto_block_offender);
        assert!(cfg.notify_on_intrusion);
    }

    #[test]
    fn test_incident_serialization() {
        let incident = HoneypotIncident {
            timestamp: 1726000000,
            source_ip: "192.168.1.99".to_string(),
            trap_port: 8080,
            blocked: true,
            description: "Test probe".to_string(),
        };

        let json = serde_json::to_string(&incident).unwrap();
        let deserialized: HoneypotIncident = serde_json::from_str(&json).unwrap();
        assert_eq!(incident, deserialized);
    }
}
