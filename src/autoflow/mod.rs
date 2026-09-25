use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[cfg(unix)]
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
    pub vpn_rules: Vec<VpnRule>,
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
            vpn_rules: vec![
                VpnRule {
                    match_name: "tailscale".to_string(),
                    profile: "dev".to_string(),
                    comment: Some("Tailscale Mesh Tailnet".to_string()),
                },
                VpnRule {
                    match_name: "wg".to_string(),
                    profile: "work".to_string(),
                    comment: Some("WireGuard Corporate Gateway".to_string()),
                },
            ],
            rules: vec![
                NetworkRule {
                    ssid: Some("HomeNetwork".to_string()),
                    bssid: None,
                    gateway_mac: None,
                    interface: None,
                    profile: "home".to_string(),
                    comment: Some("Home Wi-Fi".to_string()),
                },
                NetworkRule {
                    ssid: Some("WorkOffice_Secure".to_string()),
                    bssid: None,
                    gateway_mac: None,
                    interface: None,
                    profile: "work".to_string(),
                    comment: Some("Work Headquarters".to_string()),
                },
                NetworkRule {
                    ssid: None,
                    bssid: None,
                    gateway_mac: None,
                    interface: Some("eth0".to_string()),
                    profile: "dev".to_string(),
                    comment: Some("Wired Lab Bench".to_string()),
                },
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VpnRule {
    pub match_name: String,
    pub profile: String,
    #[serde(default)]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkRule {
    #[serde(default)]
    pub ssid: Option<String>,
    #[serde(default)]
    pub bssid: Option<String>,
    #[serde(default)]
    pub gateway_mac: Option<String>,
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
    #[serde(default)]
    pub current_bssid: Option<String>,
    #[serde(default)]
    pub current_gateway_mac: Option<String>,
    pub active_vpn: Option<String>,
    #[serde(default)]
    pub vpn_tunnels: Vec<String>,
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
                    bssid: None,
                    gateway_mac: None,
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
    #[cfg(windows)]
    {
        return crate::platform::windows::network::detect_active_networks_windows();
    }

    #[cfg(unix)]
    {
        let mut current_ssid = None;
        let mut active_vpn = None;
        let mut vpn_tunnels = Vec::new();
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

                    if con_type.contains("vpn") || con_type.contains("wireguard") || dev.starts_with("tun") || dev.starts_with("wg") || dev.starts_with("tailscale") {
                        active_vpn = Some(name.to_string());
                        vpn_tunnels.push(name.to_string());
                        if !dev.is_empty() && dev != name {
                            vpn_tunnels.push(dev.to_string());
                        }
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

        // Direct kernel network interface scan for Tailscale, WireGuard, ZeroTier, and Cloudflare WARP
        if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                let iface_name = entry.file_name().to_string_lossy().to_string();
                if iface_name.starts_with("tailscale") || iface_name.starts_with("wg") || iface_name.starts_with("zt") || iface_name.starts_with("warp") {
                    let operstate = std::fs::read_to_string(entry.path().join("operstate")).unwrap_or_default();
                    let state_str = operstate.trim();
                    if state_str == "up" || state_str == "unknown" {
                        if !active_devices.contains(&iface_name) {
                            active_devices.push(iface_name.clone());
                        }
                        if !vpn_tunnels.contains(&iface_name) {
                            vpn_tunnels.push(iface_name.clone());
                        }
                        if active_vpn.is_none() {
                            active_vpn = Some(iface_name);
                            if primary_type != "wifi" {
                                primary_type = "vpn".to_string();
                            }
                        }
                    }
                }
            }
        }

        let current_bssid = detect_current_bssid();
        let current_gateway_mac = detect_current_gateway_mac();

        ActiveNetworkInfo {
            primary_type,
            current_ssid,
            current_bssid,
            current_gateway_mac,
            active_vpn,
            vpn_tunnels,
            active_devices,
        }
    }
}

/// Detect the active Wi-Fi BSSID using nmcli
pub fn detect_current_bssid() -> Option<String> {
    #[cfg(unix)]
    {
        if let Ok(out) = Command::new("nmcli")
            .args(["-t", "-f", "active,bssid", "dev", "wifi"])
            .output()
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                for line in s.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("yes:") || trimmed.starts_with("*:") {
                        if let Some((_, bssid_part)) = trimmed.split_once(':') {
                            let clean = bssid_part.replace('\\', "").trim().to_string();
                            if clean.contains(':') && clean.len() >= 11 {
                                return Some(clean);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Detect default gateway MAC address from /proc/net/arp or ip neigh
pub fn detect_current_gateway_mac() -> Option<String> {
    #[cfg(unix)]
    {
        // 1. Direct Linux /proc/net/arp read
        if let Ok(content) = std::fs::read_to_string("/proc/net/arp") {
            let gw_ip = detect_default_gateway_ip();
            for line in content.lines().skip(1) {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if cols.len() >= 4 {
                    let ip = cols[0];
                    let mac = cols[3];
                    if mac.contains(':') && mac != "00:00:00:00:00:00" {
                        if let Some(ref target_ip) = gw_ip {
                            if ip == target_ip {
                                return Some(mac.to_string());
                            }
                        } else if cols.len() >= 6 {
                            let dev = cols[5];
                            if dev.starts_with("wl") || dev.starts_with("en") || dev.starts_with("eth") {
                                return Some(mac.to_string());
                            }
                        }
                    }
                }
            }
        }

        // 2. Fallback: ip neigh show
        if let Ok(out) = Command::new("ip").args(["neigh", "show"]).output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                for line in s.lines() {
                    if line.contains("router") || line.contains("REACHABLE") || line.contains("DELAY") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if let Some(pos) = parts.iter().position(|&x| x == "lladdr") {
                            if pos + 1 < parts.len() {
                                let mac = parts[pos + 1];
                                if mac.contains(':') {
                                    return Some(mac.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Fallback: arp -an
        if let Ok(out) = Command::new("arp").args(["-an"]).output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                for line in s.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(pos) = parts.iter().position(|&x| x == "at") {
                        if pos + 1 < parts.len() {
                            let mac = parts[pos + 1];
                            if mac.contains(':') && mac != "(incomplete)" && mac != "ff:ff:ff:ff:ff:ff" {
                                return Some(mac.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn detect_default_gateway_ip() -> Option<String> {
    #[cfg(unix)]
    {
        if let Ok(out) = Command::new("ip").args(["route", "show", "default"]).output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                let parts: Vec<&str> = s.split_whitespace().collect();
                if let Some(pos) = parts.iter().position(|&x| x == "via") {
                    if pos + 1 < parts.len() {
                        return Some(parts[pos + 1].to_string());
                    }
                }
            }
        }
    }
    None
}

/// Check if the current Wi-Fi network exhibits Evil Twin characteristics
/// (SSID matches a trusted rule, but BSSID or Gateway MAC mismatches).
pub fn check_evil_twin(config: &AutoFlowConfig, net: &ActiveNetworkInfo) -> Option<String> {
    let ssid = net.current_ssid.as_ref()?;

    for rule in &config.rules {
        if let Some(ref rule_ssid) = rule.ssid {
            if rule_ssid.eq_ignore_ascii_case(ssid) {
                // Check BSSID fingerprint
                if let Some(ref exp_bssid) = rule.bssid {
                    if let Some(ref act_bssid) = net.current_bssid {
                        let exp = exp_bssid.to_lowercase().replace('-', ":");
                        let act = act_bssid.to_lowercase().replace('-', ":");
                        if !exp.is_empty() && !act.is_empty() && exp != act {
                            return Some(format!(
                                "EVIL TWIN DETECTED: SSID '{}' advertised by rogue BSSID '{}' (expected '{}')",
                                ssid, act_bssid, exp_bssid
                            ));
                        }
                    }
                }

                // Check Gateway MAC fingerprint
                if let Some(ref exp_gw) = rule.gateway_mac {
                    if let Some(ref act_gw) = net.current_gateway_mac {
                        let exp = exp_gw.to_lowercase().replace('-', ":");
                        let act = act_gw.to_lowercase().replace('-', ":");
                        if !exp.is_empty() && !act.is_empty() && exp != act {
                            return Some(format!(
                                "EVIL TWIN DETECTED: Gateway MAC mismatch on SSID '{}' (actual '{}', expected '{}')",
                                ssid, act_gw, exp_gw
                            ));
                        }
                    }
                }
            }
        }
    }

    None
}

/// Evaluate active network against configured rules and determine appropriate posture
pub fn evaluate_posture(config: &AutoFlowConfig, net: &ActiveNetworkInfo) -> Option<String> {
    if !config.enabled {
        return None;
    }

    // 1. Specific VPN Rules (e.g. Tailscale -> dev, Corporate WireGuard -> work)
    for vpn_rule in &config.vpn_rules {
        let match_target = vpn_rule.match_name.to_lowercase();
        if let Some(ref vpn_name) = net.active_vpn {
            if vpn_name.to_lowercase().contains(&match_target) {
                return Some(vpn_rule.profile.clone());
            }
        }
        for tunnel in &net.vpn_tunnels {
            if tunnel.to_lowercase().contains(&match_target) {
                return Some(vpn_rule.profile.clone());
            }
        }
    }

    // 2. Default VPN Profile Fallback
    if net.active_vpn.is_some() || !net.vpn_tunnels.is_empty() {
        if let Some(ref vpn_prof) = config.vpn_profile {
            if !vpn_prof.is_empty() {
                return Some(vpn_prof.clone());
            }
        }
    }

    // 3. Wi-Fi SSID & Anti-Evil Twin Matching
    if let Some(ref ssid) = net.current_ssid {
        // If an Evil Twin is detected, immediately enforce travel lockdown
        if check_evil_twin(config, net).is_some() {
            return Some("travel".to_string());
        }

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

    // 4. Interface matching (e.g. eth0, docker0)
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
            bssid: None,
            gateway_mac: None,
            interface: None,
            profile: "travel".to_string(),
            comment: None,
        });

        // Test matched SSID
        let net1 = ActiveNetworkInfo {
            primary_type: "wifi".to_string(),
            current_ssid: Some("CoffeeShop-Guest".to_string()),
            current_bssid: None,
            current_gateway_mac: None,
            active_vpn: None,
            vpn_tunnels: Vec::new(),
            active_devices: vec!["wlp2s0".to_string()],
        };
        assert_eq!(evaluate_posture(&config, &net1), Some("travel".to_string()));

        // Test VPN override
        let net2 = ActiveNetworkInfo {
            primary_type: "vpn".to_string(),
            current_ssid: Some("HomeNetwork".to_string()),
            current_bssid: None,
            current_gateway_mac: None,
            active_vpn: Some("WorkVPN".to_string()),
            vpn_tunnels: vec!["tun0".to_string()],
            active_devices: vec!["tun0".to_string()],
        };
        assert_eq!(evaluate_posture(&config, &net2), Some("work".to_string()));

        // Test unknown Wi-Fi fallback to travel
        let net3 = ActiveNetworkInfo {
            primary_type: "wifi".to_string(),
            current_ssid: Some("Airport-Public-Free".to_string()),
            current_bssid: None,
            current_gateway_mac: None,
            active_vpn: None,
            vpn_tunnels: Vec::new(),
            active_devices: vec!["wlp2s0".to_string()],
        };
        assert_eq!(evaluate_posture(&config, &net3), Some("travel".to_string()));

        // Test Tailscale routing directly to dev profile
        let net4 = ActiveNetworkInfo {
            primary_type: "vpn".to_string(),
            current_ssid: Some("HomeNetwork".to_string()),
            current_bssid: None,
            current_gateway_mac: None,
            active_vpn: Some("tailscale0".to_string()),
            vpn_tunnels: vec!["tailscale0".to_string()],
            active_devices: vec!["wlp2s0".to_string(), "tailscale0".to_string()],
        };
        assert_eq!(evaluate_posture(&config, &net4), Some("dev".to_string()));
    }

    #[test]
    fn test_anti_evil_twin_bssid_mismatch_locks_to_travel() {
        let mut config = AutoFlowConfig::default();
        config.rules.push(NetworkRule {
            ssid: Some("CorporateOffice".to_string()),
            bssid: Some("aa:bb:cc:dd:ee:ff".to_string()),
            gateway_mac: None,
            interface: None,
            profile: "work".to_string(),
            comment: Some("Legit AP".to_string()),
        });

        // 1. Matching BSSID: allows work profile
        let legit_net = ActiveNetworkInfo {
            primary_type: "wifi".to_string(),
            current_ssid: Some("CorporateOffice".to_string()),
            current_bssid: Some("AA:BB:CC:DD:EE:FF".to_string()),
            current_gateway_mac: None,
            active_vpn: None,
            vpn_tunnels: Vec::new(),
            active_devices: vec!["wlp2s0".to_string()],
        };
        assert!(check_evil_twin(&config, &legit_net).is_none());
        assert_eq!(evaluate_posture(&config, &legit_net), Some("work".to_string()));

        // 2. Rogue BSSID: triggers evil twin detection and forces travel lockdown
        let rogue_net = ActiveNetworkInfo {
            primary_type: "wifi".to_string(),
            current_ssid: Some("CorporateOffice".to_string()),
            current_bssid: Some("11:22:33:44:55:66".to_string()),
            current_gateway_mac: None,
            active_vpn: None,
            vpn_tunnels: Vec::new(),
            active_devices: vec!["wlp2s0".to_string()],
        };
        let alert = check_evil_twin(&config, &rogue_net);
        assert!(alert.is_some());
        assert!(alert.unwrap().contains("rogue BSSID '11:22:33:44:55:66'"));
        assert_eq!(evaluate_posture(&config, &rogue_net), Some("travel".to_string()));
    }

    #[test]
    fn test_anti_evil_twin_gateway_mac_mismatch_locks_to_travel() {
        let mut config = AutoFlowConfig::default();
        config.rules.push(NetworkRule {
            ssid: Some("HomeNetwork".to_string()),
            bssid: None,
            gateway_mac: Some("00:11:22:33:44:55".to_string()),
            interface: None,
            profile: "home".to_string(),
            comment: Some("Trusted router".to_string()),
        });

        // Rogue gateway MAC on legitimate SSID
        let rogue_gw = ActiveNetworkInfo {
            primary_type: "wifi".to_string(),
            current_ssid: Some("HomeNetwork".to_string()),
            current_bssid: None,
            current_gateway_mac: Some("99:88:77:66:55:44".to_string()),
            active_vpn: None,
            vpn_tunnels: Vec::new(),
            active_devices: vec!["wlp2s0".to_string()],
        };
        assert!(check_evil_twin(&config, &rogue_gw).is_some());
        assert_eq!(evaluate_posture(&config, &rogue_gw), Some("travel".to_string()));
    }
}
