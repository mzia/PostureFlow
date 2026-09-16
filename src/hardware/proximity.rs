use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProximityConfig {
    #[serde(default = "default_false")]
    pub enabled: bool,
    #[serde(default)]
    pub target_device_mac: Option<String>,
    #[serde(default)]
    pub target_device_name: Option<String>,
    #[serde(default = "default_lock_threshold")]
    pub rssi_lock_threshold: i32,
    #[serde(default = "default_unlock_threshold")]
    pub rssi_unlock_threshold: i32,
    #[serde(default = "default_grace_period")]
    pub grace_period_seconds: u32,
    #[serde(default = "default_true")]
    pub auto_lock_screen: bool,
    #[serde(default = "default_lock_profile")]
    pub lock_profile: String,
}

fn default_false() -> bool {
    false
}

fn default_true() -> bool {
    true
}

fn default_lock_threshold() -> i32 {
    -85
}

fn default_unlock_threshold() -> i32 {
    -70
}

fn default_grace_period() -> u32 {
    10
}

fn default_lock_profile() -> String {
    "travel".to_string()
}

impl Default for ProximityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            target_device_mac: None,
            target_device_name: None,
            rssi_lock_threshold: -85,
            rssi_unlock_threshold: -70,
            grace_period_seconds: 10,
            auto_lock_screen: true,
            lock_profile: "travel".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProximityAction {
    LockAndDemote { lock_profile: String },
    Restore { restore_profile: String },
}

/// Discover paired Bluetooth devices (MAC, Name) via bluetoothctl
pub fn list_paired_bluetooth_devices() -> Vec<(String, String)> {
    let mut devices = Vec::new();

    #[cfg(unix)]
    {
        if let Ok(out) = crate::system::execute("bluetoothctl", &["devices"]) {
            for line in out.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 && parts[0] == "Device" {
                    let mac = parts[1].to_string();
                    let name = parts[2..].join(" ");
                    devices.push((mac, name));
                }
            }
        }
    }

    devices
}

/// Query current RSSI signal strength for a specific Bluetooth device
pub fn query_device_rssi(mac: &str) -> Option<i32> {
    #[cfg(unix)]
    {
        if let Ok(out) = crate::system::execute("bluetoothctl", &["info", mac]) {
            for line in out.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("RSSI:") {
                    if let Ok(val) = rest.trim().parse::<i32>() {
                        return Some(val);
                    }
                }
            }
        }
    }
    let _ = mac;
    None
}

/// Evaluate proximity distance transitions based on RSSI and grace period
pub fn evaluate_proximity_transition(
    rssi_opt: Option<i32>,
    consecutive_out_of_range: u32,
    config: &ProximityConfig,
    current_profile: &str,
    previously_locked: bool,
) -> (Option<ProximityAction>, u32, bool) {
    if !config.enabled || config.target_device_mac.is_none() {
        return (None, 0, false);
    }

    let is_out_of_range = match rssi_opt {
        Some(rssi) => rssi < config.rssi_lock_threshold,
        None => true, // device completely unreachable
    };

    if is_out_of_range {
        let new_count = consecutive_out_of_range + 1;
        // Check if grace period exceeded (assume 2-second polling tick, count >= grace / 2)
        let ticks_needed = (config.grace_period_seconds / 2).max(1);
        if new_count >= ticks_needed && !previously_locked {
            if current_profile != config.lock_profile {
                return (
                    Some(ProximityAction::LockAndDemote {
                        lock_profile: config.lock_profile.clone(),
                    }),
                    new_count,
                    true,
                );
            }
        }
        (None, new_count, previously_locked)
    } else {
        // Device is back in strong range
        if let Some(rssi) = rssi_opt {
            if rssi >= config.rssi_unlock_threshold && previously_locked {
                return (
                    Some(ProximityAction::Restore {
                        restore_profile: "work".to_string(),
                    }),
                    0,
                    false,
                );
            }
        }
        (None, 0, previously_locked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proximity_default_config() {
        let cfg = ProximityConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.rssi_lock_threshold, -85);
        assert_eq!(cfg.rssi_unlock_threshold, -70);
        assert_eq!(cfg.grace_period_seconds, 10);
        assert_eq!(cfg.lock_profile, "travel");
    }

    #[test]
    fn test_walk_away_triggers_lock_after_grace_period() {
        let mut cfg = ProximityConfig::default();
        cfg.enabled = true;
        cfg.target_device_mac = Some("AA:BB:CC:DD:EE:FF".to_string());
        cfg.grace_period_seconds = 4; // 2 ticks needed

        // Tick 1: out of range, count becomes 1, no action
        let (action, count, locked) = evaluate_proximity_transition(Some(-90), 0, &cfg, "work", false);
        assert_eq!(action, None);
        assert_eq!(count, 1);
        assert!(!locked);

        // Tick 2: out of range, count becomes 2 >= 2, triggers lock
        let (action, count, locked) = evaluate_proximity_transition(Some(-92), count, &cfg, "work", locked);
        assert_eq!(
            action,
            Some(ProximityAction::LockAndDemote {
                lock_profile: "travel".to_string()
            })
        );
        assert_eq!(count, 2);
        assert!(locked);
    }

    #[test]
    fn test_return_in_range_triggers_restore() {
        let mut cfg = ProximityConfig::default();
        cfg.enabled = true;
        cfg.target_device_mac = Some("AA:BB:CC:DD:EE:FF".to_string());

        let (action, count, locked) = evaluate_proximity_transition(Some(-65), 5, &cfg, "travel", true);
        assert_eq!(
            action,
            Some(ProximityAction::Restore {
                restore_profile: "work".to_string()
            })
        );
        assert_eq!(count, 0);
        assert!(!locked);
    }
}
