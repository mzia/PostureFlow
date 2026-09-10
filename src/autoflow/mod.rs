use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoFlowConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_travel")]
    pub default_unknown_wifi: String,
    #[serde(default)]
    pub vpn_profile: Option<String>,
    #[serde(default)]
    pub rules: Vec<NetworkRule>,
}

fn default_true() -> bool {
    true
}

fn default_travel() -> String {
    "travel".to_string()
}

impl Default for AutoFlowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_unknown_wifi: "travel".to_string(),
            vpn_profile: Some("work".to_string()),
            rules: vec![
                NetworkRule {
                    ssid: Some("HomeNetwork".to_string()),
                    interface: None,
                    profile: "home".to_string(),
                    comment: Some("Home Wi-Fi".to_string()),
                },
                NetworkRule {
                    ssid: Some("WorkOffice_Secure".to_string()),
                    interface: None,
                    profile: "work".to_string(),
                    comment: Some("Work Headquarters".to_string()),
                },
                NetworkRule {
                    ssid: None,
                    interface: Some("eth0".to_string()),
                    profile: "dev".to_string(),
                    comment: Some("Wired Lab Bench".to_string()),
                },
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkRule {
    #[serde(default)]
    pub ssid: Option<String>,
    #[serde(default)]
    pub interface: Option<String>,
    pub profile: String,
    #[serde(default)]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveNetworkInfo {
    pub primary_type: String,            // "wifi", "ethernet", "vpn", "none"
    pub current_ssid: Option<String>,
    pub active_vpn: Option<String>,
    pub active_devices: Vec<String>,
}

pub fn primary_config_path() -> PathBuf {
    PathBuf::from("/etc/postureflow/autoflow.toml")
}

pub fn user_config_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/postureflow/autoflow.toml")
    } else {
        PathBuf::from("/tmp/postureflow-autoflow.toml")
    }
}

pub fn load_autoflow_config() -> AutoFlowConfig {
    let p_path = primary_config_path();
    if p_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&p_path) {
            if let Ok(cfg) = toml::from_str::<AutoFlowConfig>(&content) {
                return cfg;
            }
        }
    }

    let u_path = user_config_path();
    if u_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&u_path) {
            if let Ok(cfg) = toml::from_str::<AutoFlowConfig>(&content) {
                return cfg;
            }
        }
    }

    // Default template with current detected SSID pre-populated if available
    let mut default_cfg = AutoFlowConfig::default();
    let current_net = detect_active_networks();
    if let Some(ssid) = current_net.current_ssid {
        if !default_cfg.rules.iter().any(|r| r.ssid.as_deref() == Some(&ssid)) {
            default_cfg.rules.insert(
                0,
                NetworkRule {
                    ssid: Some(ssid.clone()),
                    interface: None,
                    profile: "home".to_string(),
                    comment: Some(format!("Current Wi-Fi: {}", ssid)),
                },
            );
        }
    }
    default_cfg
}

pub fn save_autoflow_config(config: &AutoFlowConfig) -> Result<PathBuf, String> {
    let toml_str = toml::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize TOML: {}", e))?;

    let p_path = primary_config_path();
    // Try system path first if parent dir exists and is writable
    if let Some(parent) = p_path.parent() {
        if parent.exists() && std::fs::write(&p_path, &toml_str).is_ok() {
            return Ok(p_path);
        }
    }

    // Fallback to user path
    let u_path = user_config_path();
    if let Some(parent) = u_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&u_path, &toml_str)
        .map_err(|e| format!("Failed to write to {}: {}", u_path.display(), e))?;

    Ok(u_path)
}

/// Detect the active network environment using nmcli
pub fn detect_active_networks() -> ActiveNetworkInfo {
    let mut current_ssid = None;
    let mut active_vpn = None;
    let mut active_devices = Vec::new();
    let mut primary_type = "none".to_string();

    let output = Command::new("nmcli")
        .args(["-t", "-f", "TYPE,NAME,DEVICE", "con", "show", "--active"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() < 3 {
                    continue;
                }
                let con_type = parts[0].to_lowercase();
                let name = parts[1];
                let dev = parts[2];

                if con_type == "loopback" {
                    continue;
                }

                active_devices.push(dev.to_string());

                if con_type.contains("vpn") || con_type.contains("wireguard") || dev.starts_with("tun") || dev.starts_with("wg") {
                    active_vpn = Some(name.to_string());
                    if primary_type != "wifi" {
                        primary_type = "vpn".to_string();
                    }
                } else if con_type.contains("wireless") || con_type.contains("wifi") || con_type.contains("802-11") {
                    current_ssid = Some(name.to_string());
                    primary_type = "wifi".to_string();
                } else if con_type.contains("ethernet") || dev.starts_with("en") || dev.starts_with("eth") {
                    if primary_type == "none" {
                        primary_type = "ethernet".to_string();
                    }
                }
            }
        }
    }

    ActiveNetworkInfo {
        primary_type,
        current_ssid,
        active_vpn,
        active_devices,
    }
}

/// Evaluate active network against configured rules and determine appropriate posture
pub fn evaluate_posture(config: &AutoFlowConfig, net: &ActiveNetworkInfo) -> Option<String> {
    if !config.enabled {
        return None;
    }

    // 1. VPN Rule Priority
    if net.active_vpn.is_some() {
        if let Some(ref vpn_prof) = config.vpn_profile {
            if !vpn_prof.is_empty() {
                return Some(vpn_prof.clone());
            }
        }
    }

    // 2. Wi-Fi SSID Matching
    if let Some(ref ssid) = net.current_ssid {
        for rule in &config.rules {
            if let Some(ref rule_ssid) = rule.ssid {
                if rule_ssid.eq_ignore_ascii_case(ssid) {
                    return Some(rule.profile.clone());
                }
            }
        }

        // Unknown Wi-Fi connected! Default to untrusted/travel posture
        if !config.default_unknown_wifi.is_empty() {
            return Some(config.default_unknown_wifi.clone());
        }
    }

    // 3. Interface matching (e.g. eth0, docker0)
    for dev in &net.active_devices {
        for rule in &config.rules {
            if let Some(ref rule_iface) = rule.interface {
                if rule_iface.eq_ignore_ascii_case(dev) {
                    return Some(rule.profile.clone());
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autoflow_rule_evaluation() {
        let mut config = AutoFlowConfig::default();
        config.rules.push(NetworkRule {
            ssid: Some("CoffeeShop-Guest".to_string()),
            interface: None,
            profile: "travel".to_string(),
            comment: None,
        });

        // Test matched SSID
        let net1 = ActiveNetworkInfo {
            primary_type: "wifi".to_string(),
            current_ssid: Some("CoffeeShop-Guest".to_string()),
            active_vpn: None,
            active_devices: vec!["wlp2s0".to_string()],
        };
        assert_eq!(evaluate_posture(&config, &net1), Some("travel".to_string()));

        // Test VPN override
        let net2 = ActiveNetworkInfo {
            primary_type: "vpn".to_string(),
            current_ssid: Some("HomeNetwork".to_string()),
            active_vpn: Some("WorkVPN".to_string()),
            active_devices: vec!["tun0".to_string()],
        };
        assert_eq!(evaluate_posture(&config, &net2), Some("work".to_string()));

        // Test unknown Wi-Fi fallback to travel
        let net3 = ActiveNetworkInfo {
            primary_type: "wifi".to_string(),
            current_ssid: Some("Airport-Public-Free".to_string()),
            active_vpn: None,
            active_devices: vec!["wlp2s0".to_string()],
        };
        assert_eq!(evaluate_posture(&config, &net3), Some("travel".to_string()));
    }
}
