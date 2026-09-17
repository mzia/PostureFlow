pub mod yubikey;
pub mod honeypot;
pub mod sensors;
pub mod proximity;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const SYSTEM_HARDWARE_CONFIG: &str = "/etc/postureflow/hardware.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct HardwareDefenseConfig {
    #[serde(default)]
    pub yubikey: yubikey::YubikeyTetherConfig,
    #[serde(default)]
    pub honeypot: honeypot::HoneypotConfig,
    #[serde(default)]
    pub sensors: sensors::SensorPrivacyConfig,
    #[serde(default)]
    pub proximity: proximity::ProximityConfig,
}

impl HardwareDefenseConfig {
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }
}

pub fn user_hardware_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config/postureflow/hardware.toml")
}

pub fn load_hardware_config() -> HardwareDefenseConfig {
    // 1. User config override
    let user_path = user_hardware_config_path();
    if user_path.exists() {
        if let Ok(content) = fs::read_to_string(&user_path) {
            if let Ok(cfg) = HardwareDefenseConfig::from_toml(&content) {
                return cfg;
            }
        }
    }

    // 2. System config
    let sys_path = Path::new(SYSTEM_HARDWARE_CONFIG);
    if sys_path.exists() {
        if let Ok(content) = fs::read_to_string(sys_path) {
            if let Ok(cfg) = HardwareDefenseConfig::from_toml(&content) {
                return cfg;
            }
        }
    }

    // 3. Fallback default
    HardwareDefenseConfig::default()
}

pub fn save_hardware_config(
    config: &HardwareDefenseConfig,
    privileged: bool,
) -> Result<(), String> {
    let toml = config.to_toml().map_err(|e| format!("Failed to serialize hardware config: {}", e))?;

    let path = if privileged {
        PathBuf::from(SYSTEM_HARDWARE_CONFIG)
    } else {
        user_hardware_config_path()
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    fs::write(&path, toml).map_err(|e| format!("Failed to write hardware config to {:?}: {}", path, e))?;
    Ok(())
}

/// Start background monitor loop managing physical YubiKey tethering, honeypot traps, and proximity
pub async fn run_hardware_defense_monitor(shutdown: Arc<AtomicBool>) {
    println!("[+] Hardware Defense & Physical Token background monitor activated.");

    let mut was_yubikey_present = yubikey::is_yubikey_present(None);
    let mut consecutive_out_of_range: u32 = 0;
    let mut proximity_locked = false;

    // Spawn honeypot listeners if configured
    let config = load_hardware_config();
    if config.honeypot.enabled {
        for port in &config.honeypot.trap_ports {
            let p = *port;
            let auto_block = config.honeypot.auto_block_offender;
            let notify = config.honeypot.notify_on_intrusion;
            let s = Arc::clone(&shutdown);
            tokio::spawn(async move {
                honeypot::run_port_trap_listener(p, auto_block, notify, s).await;
            });
        }
    }

    while !shutdown.load(Ordering::Relaxed) {
        let cfg = load_hardware_config();
        let current_profile = crate::system::get_active_profile();

        // 1. YubiKey Hardware Presence Tethering Check
        if cfg.yubikey.enabled {
            let is_present = yubikey::is_yubikey_present(cfg.yubikey.target_serial.as_deref());
            if let Some(action) = yubikey::evaluate_tether_transition(
                was_yubikey_present,
                is_present,
                &cfg.yubikey,
                &current_profile,
            ) {
                yubikey::execute_tether_action(action);
            }
            was_yubikey_present = is_present;
        }

        // 2. Bluetooth RSSI Walk-Away Proximity Check
        if cfg.proximity.enabled && cfg.proximity.target_device_mac.is_some() {
            let target_mac = cfg.proximity.target_device_mac.as_deref().unwrap();
            let rssi = proximity::query_device_rssi(target_mac);
            let (action, new_count, new_locked) = proximity::evaluate_proximity_transition(
                rssi,
                consecutive_out_of_range,
                &cfg.proximity,
                &current_profile,
                proximity_locked,
            );
            consecutive_out_of_range = new_count;
            proximity_locked = new_locked;

            if let Some(act) = action {
                match act {
                    proximity::ProximityAction::LockAndDemote { lock_profile } => {
                        eprintln!(
                            "[!] 🛰️ Walk-Away detected: Bluetooth signal lost. Locking session & engaging [{}]...",
                            lock_profile.to_uppercase()
                        );
                        let _ = crate::system::lock_desktop_session();
                        let _ = crate::system::apply_profile_by_id(&lock_profile);
                        let _ = crate::system::execute(
                            "notify-send",
                            &[
                                "-u",
                                "critical",
                                "PostureFlow Walk-Away Security",
                                &format!("🛰️ Bluetooth device out of range: Locked session and engaged {} posture.", lock_profile.to_uppercase()),
                            ],
                        );
                    }
                    proximity::ProximityAction::Restore { restore_profile } => {
                        println!(
                            "[+] 🛰️ Bluetooth proximity re-acquired. Restoring [{}] profile...",
                            restore_profile.to_uppercase()
                        );
                        let _ = crate::system::apply_profile_by_id(&restore_profile);
                        let _ = crate::system::execute(
                            "notify-send",
                            &[
                                "-u",
                                "normal",
                                "PostureFlow Walk-Away Security",
                                &format!("🛰️ Bluetooth device in range: Restored {} posture.", restore_profile.to_uppercase()),
                            ],
                        );
                    }
                }
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    }

    println!("[*] Hardware defense monitor shut down.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_defense_config_defaults() {
        let cfg = HardwareDefenseConfig::default();
        assert!(!cfg.yubikey.enabled);
        assert!(!cfg.honeypot.enabled);
        assert!(!cfg.sensors.camera_blocked);
        assert!(!cfg.proximity.enabled);
    }

    #[test]
    fn test_hardware_defense_config_toml_roundtrip() {
        let mut cfg = HardwareDefenseConfig::default();
        cfg.yubikey.enabled = true;
        cfg.honeypot.enabled = true;
        cfg.honeypot.trap_ports = vec![2222, 9090];
        cfg.sensors.camera_blocked = true;
        cfg.proximity.enabled = true;
        cfg.proximity.target_device_mac = Some("11:22:33:44:55:66".to_string());

        let toml = cfg.to_toml().unwrap();
        let loaded = HardwareDefenseConfig::from_toml(&toml).unwrap();
        assert_eq!(cfg, loaded);
    }
}
