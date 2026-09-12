use std::process::Command;
use crate::autoflow::ActiveNetworkInfo;

pub fn detect_active_networks_windows() -> ActiveNetworkInfo {
    let mut current_ssid = None;
    let mut active_vpn = None;
    let mut active_devices = Vec::new();
    let mut primary_type = "none".to_string();

    // 1. Query Wi-Fi interfaces via netsh
    if let Ok(output) = Command::new("netsh").args(["wlan", "show", "interfaces"]).output() {
        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            for line in out_str.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("SSID") && !trimmed.starts_with("BSSID") {
                    if let Some((_, val)) = trimmed.split_once(':') {
                        let ssid = val.trim().to_string();
                        if !ssid.is_empty() {
                            current_ssid = Some(ssid);
                            primary_type = "wifi".to_string();
                        }
                    }
                }
            }
        }
    }

    // 2. Query network adapters via netsh
    if let Ok(output) = Command::new("netsh").args(["interface", "show", "interface"]).output() {
        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            for line in out_str.lines() {
                let lower = line.to_lowercase();
                if lower.contains("connected") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(name) = parts.last() {
                        let dev = name.to_string();
                        active_devices.push(dev.clone());
                        if lower.contains("vpn") || lower.contains("wireguard") || lower.contains("tailscale") {
                            active_vpn = Some(dev);
                            if primary_type != "wifi" {
                                primary_type = "vpn".to_string();
                            }
                        } else if lower.contains("ethernet") && primary_type == "none" {
                            primary_type = "ethernet".to_string();
                        }
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
