use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const YUBICO_VENDOR_ID: &str = "1050";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct YubikeyDevice {
    pub vendor_id: String,
    pub product_id: String,
    pub product_name: String,
    pub serial: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct YubikeyTetherConfig {
    #[serde(default = "default_false")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub lock_session_on_removal: bool,
    #[serde(default = "default_fallback_profile")]
    pub fallback_profile: String,
    #[serde(default = "default_true")]
    pub restore_profile_on_insert: bool,
    #[serde(default = "default_preferred_profile")]
    pub preferred_profile: String,
    #[serde(default)]
    pub target_serial: Option<String>,
}

fn default_false() -> bool {
    false
}

fn default_true() -> bool {
    true
}

fn default_fallback_profile() -> String {
    "travel".to_string()
}

fn default_preferred_profile() -> String {
    "work".to_string()
}

impl Default for YubikeyTetherConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            lock_session_on_removal: true,
            fallback_profile: "travel".to_string(),
            restore_profile_on_insert: true,
            preferred_profile: "work".to_string(),
            target_serial: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TetherAction {
    LockAndDemote { fallback_profile: String },
    Restore { preferred_profile: String },
}

/// Detect all connected YubiKey devices via USB sysfs
pub fn detect_yubikeys() -> Vec<YubikeyDevice> {
    let mut devices = Vec::new();

    #[cfg(unix)]
    {
        let sysfs_dir = Path::new("/sys/bus/usb/devices");
        if let Ok(entries) = fs::read_dir(sysfs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let vendor_file = path.join("idVendor");
                if let Ok(vendor) = fs::read_to_string(&vendor_file) {
                    if vendor.trim() == YUBICO_VENDOR_ID {
                        let product_id = fs::read_to_string(path.join("idProduct"))
                            .map(|s| s.trim().to_string())
                            .unwrap_or_else(|_| "unknown".to_string());
                        let product_name = fs::read_to_string(path.join("product"))
                            .map(|s| s.trim().to_string())
                            .unwrap_or_else(|_| "YubiKey".to_string());
                        let serial = fs::read_to_string(path.join("serial"))
                            .map(|s| s.trim().to_string())
                            .ok();

                        devices.push(YubikeyDevice {
                            vendor_id: YUBICO_VENDOR_ID.to_string(),
                            product_id,
                            product_name,
                            serial,
                        });
                    }
                }
            }
        }
    }

    devices
}

/// Returns true if a YubiKey (optionally matching a target serial number) is currently connected
pub fn is_yubikey_present(target_serial: Option<&str>) -> bool {
    let keys = detect_yubikeys();
    if keys.is_empty() {
        return false;
    }
    match target_serial {
        Some(expected_serial) => keys.iter().any(|k| k.serial.as_deref() == Some(expected_serial)),
        None => true,
    }
}

/// Evaluates tether state transitions when a presence change is detected
pub fn evaluate_tether_transition(
    was_present: bool,
    is_present: bool,
    config: &YubikeyTetherConfig,
    current_profile: &str,
) -> Option<TetherAction> {
    if !config.enabled {
        return None;
    }

    if was_present && !is_present {
        // YubiKey was unplugged -> Lock session & demote to lockdown profile
        if current_profile != config.fallback_profile {
            return Some(TetherAction::LockAndDemote {
                fallback_profile: config.fallback_profile.clone(),
            });
        }
    } else if !was_present && is_present && config.restore_profile_on_insert {
        // YubiKey was plugged back in -> Restore preferred profile
        if current_profile == config.fallback_profile {
            return Some(TetherAction::Restore {
                preferred_profile: config.preferred_profile.clone(),
            });
        }
    }

    None
}

/// Executes a tether transition action
pub fn execute_tether_action(action: TetherAction) {
    match action {
        TetherAction::LockAndDemote { fallback_profile } => {
            eprintln!(
                "[!] 🔑 YubiKey hardware token detached! Locking session and switching to [{}]...",
                fallback_profile.to_uppercase()
            );
            let _ = crate::system::lock_desktop_session();
            let _ = crate::system::apply_profile_by_id(&fallback_profile);
            let _ = crate::system::execute(
                "notify-send",
                &[
                    "-u",
                    "critical",
                    "PostureFlow Hardware Security",
                    "🔑 YubiKey detached: Session locked and Travel lockdown engaged.",
                ],
            );
        }
        TetherAction::Restore { preferred_profile } => {
            println!(
                "[+] 🔑 YubiKey hardware token detected. Restoring [{}] profile...",
                preferred_profile.to_uppercase()
            );
            let _ = crate::system::apply_profile_by_id(&preferred_profile);
            let _ = crate::system::execute(
                "notify-send",
                &[
                    "-u",
                    "normal",
                    "PostureFlow Hardware Security",
                    &format!("🔑 YubiKey detected: Restored {} posture.", preferred_profile.to_uppercase()),
                ],
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yubikey_tether_default_config() {
        let cfg = YubikeyTetherConfig::default();
        assert!(!cfg.enabled);
        assert!(cfg.lock_session_on_removal);
        assert_eq!(cfg.fallback_profile, "travel");
        assert_eq!(cfg.preferred_profile, "work");
        assert!(cfg.restore_profile_on_insert);
    }

    #[test]
    fn test_tether_removal_triggers_demote() {
        let mut cfg = YubikeyTetherConfig::default();
        cfg.enabled = true;
        cfg.fallback_profile = "travel".to_string();

        let action = evaluate_tether_transition(true, false, &cfg, "work");
        assert_eq!(
            action,
            Some(TetherAction::LockAndDemote {
                fallback_profile: "travel".to_string()
            })
        );
    }

    #[test]
    fn test_tether_insertion_triggers_restore() {
        let mut cfg = YubikeyTetherConfig::default();
        cfg.enabled = true;
        cfg.fallback_profile = "travel".to_string();
        cfg.preferred_profile = "dev".to_string();

        let action = evaluate_tether_transition(false, true, &cfg, "travel");
        assert_eq!(
            action,
            Some(TetherAction::Restore {
                preferred_profile: "dev".to_string()
            })
        );
    }

    #[test]
    fn test_disabled_tether_produces_no_action() {
        let cfg = YubikeyTetherConfig::default(); // enabled = false
        let action = evaluate_tether_transition(true, false, &cfg, "work");
        assert_eq!(action, None);
    }
}
