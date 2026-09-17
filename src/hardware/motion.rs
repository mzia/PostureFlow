use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AccelVector {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl AccelVector {
    pub fn distance_to(&self, other: &Self) -> u32 {
        let dx = (self.x - other.x) as f64;
        let dy = (self.y - other.y) as f64;
        let dz = (self.z - other.z) as f64;
        (dx * dx + dy * dy + dz * dz).sqrt().round() as u32
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MotionSentryConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_false")]
    pub armed: bool,
    #[serde(default = "default_sensitivity")]
    pub sensitivity: u32,
    #[serde(default = "default_true")]
    pub auto_arm_on_travel: bool,
    #[serde(default = "default_true")]
    pub trigger_alarm_sound: bool,
    #[serde(default = "default_true")]
    pub auto_lock_screen: bool,
    #[serde(default = "default_true")]
    pub auto_engage_travel: bool,
}

fn default_false() -> bool {
    false
}

fn default_true() -> bool {
    true
}

fn default_sensitivity() -> u32 {
    2000 // Reasonable physical delta threshold
}

impl Default for MotionSentryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            armed: false,
            sensitivity: default_sensitivity(),
            auto_arm_on_travel: true,
            trigger_alarm_sound: true,
            auto_lock_screen: true,
            auto_engage_travel: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MotionAction {
    TriggerAlarm {
        displacement: u32,
        current: AccelVector,
        baseline: AccelVector,
    },
}

/// Find the primary IIO accelerometer sysfs path
pub fn find_accelerometer_device() -> Option<PathBuf> {
    let iio_dir = Path::new("/sys/bus/iio/devices");
    if !iio_dir.exists() {
        return None;
    }

    if let Ok(entries) = fs::read_dir(iio_dir) {
        let mut candidates = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.join("in_accel_x_raw").exists() {
                // If it has "name", check for cros-ec-accel or accel
                let name = fs::read_to_string(path.join("name")).unwrap_or_default();
                candidates.push((path, name));
            }
        }

        // Prioritize base sensor or cros-ec-accel
        if let Some((best, _)) = candidates.iter().find(|(_, name)| name.contains("accel")) {
            return Some(best.clone());
        }

        if let Some((first, _)) = candidates.first() {
            return Some(first.clone());
        }
    }

    None
}

/// Read current raw 3-axis accelerometer vector
pub fn read_accelerometer_vector() -> Option<AccelVector> {
    let dev = find_accelerometer_device()?;

    let x: i32 = fs::read_to_string(dev.join("in_accel_x_raw"))
        .ok()?
        .trim()
        .parse()
        .ok()?;
    let y: i32 = fs::read_to_string(dev.join("in_accel_y_raw"))
        .ok()?
        .trim()
        .parse()
        .ok()?;
    let z: i32 = fs::read_to_string(dev.join("in_accel_z_raw"))
        .ok()?
        .trim()
        .parse()
        .ok()?;

    Some(AccelVector { x, y, z })
}

/// Evaluate motion sensor delta against baseline
pub fn evaluate_motion_transition(
    current: AccelVector,
    baseline: AccelVector,
    config: &MotionSentryConfig,
) -> Option<MotionAction> {
    if !config.enabled || !config.armed {
        return None;
    }

    let delta = current.distance_to(&baseline);
    if delta >= config.sensitivity {
        Some(MotionAction::TriggerAlarm {
            displacement: delta,
            current,
            baseline,
        })
    } else {
        None
    }
}

/// Execute anti-theft motion sentry response
pub fn execute_motion_action(action: MotionAction, config: &MotionSentryConfig) {
    match action {
        MotionAction::TriggerAlarm {
            displacement,
            current,
            baseline: _,
        } => {
            eprintln!(
                "[!] 🚨 MOTION SENTRY ALARM: Physical displacement detected (Δ = {}, current: [{}, {}, {}])!",
                displacement, current.x, current.y, current.z
            );

            // 1. Lock screen immediately
            if config.auto_lock_screen {
                let _ = crate::system::lock_desktop_session();
            }

            // 2. Engage travel stealth lockdown
            if config.auto_engage_travel {
                let _ = crate::system::apply_profile_by_id("travel");
            }

            // 3. Play alarm tone
            if config.trigger_alarm_sound {
                play_alarm_tone();
            }

            // 4. Record security incident
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            super::honeypot::record_incident(super::honeypot::HoneypotIncident {
                timestamp: now,
                source_ip: "FRAMEWORK ACCELEROMETER".to_string(),
                trap_port: 0,
                blocked: true,
                description: format!(
                    "🚨 Physical Displacement: Laptop moved/tilted (Δ = {}, Threshold = {})",
                    displacement, config.sensitivity
                ),
            });

            // 5. Desktop Notification
            let _ = crate::system::execute(
                "notify-send",
                &[
                    "-u",
                    "critical",
                    "PostureFlow Anti-Theft Sentry",
                    &format!(
                        "🚨 Physical motion detected (Δ = {})! Laptop locked and Travel mode engaged.",
                        displacement
                    ),
                ],
            );
        }
    }
}

/// Play audio alarm sound using available system utilities
pub fn play_alarm_tone() {
    let sounds = [
        "/usr/share/sounds/Pop/stereo/notification/battery-caution.oga",
        "/usr/share/sounds/Pop/stereo/action/bell.oga",
        "/usr/share/sounds/freedesktop/stereo/alarm-clock-elapsed.oga",
    ];

    for s in &sounds {
        if Path::new(s).exists() {
            if Command::new("pw-play").arg(s).spawn().is_ok() {
                return;
            }
            if Command::new("paplay").arg(s).spawn().is_ok() {
                return;
            }
            if Command::new("aplay").arg(s).spawn().is_ok() {
                return;
            }
        }
    }

    // Fallback: system bell character
    eprint!("\x07\x07\x07");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_calculation() {
        let v1 = AccelVector { x: 0, y: 0, z: 0 };
        let v2 = AccelVector { x: 300, y: 400, z: 0 };
        assert_eq!(v1.distance_to(&v2), 500);
    }

    #[test]
    fn test_motion_evaluation_disarmed() {
        let v1 = AccelVector { x: 0, y: 0, z: 0 };
        let v2 = AccelVector { x: 3000, y: 4000, z: 0 };
        let cfg = MotionSentryConfig {
            enabled: true,
            armed: false,
            sensitivity: 1000,
            ..Default::default()
        };
        assert!(evaluate_motion_transition(v2, v1, &cfg).is_none());
    }

    #[test]
    fn test_motion_evaluation_armed_trigger() {
        let v1 = AccelVector { x: 0, y: 0, z: 0 };
        let v2 = AccelVector { x: 3000, y: 4000, z: 0 };
        let cfg = MotionSentryConfig {
            enabled: true,
            armed: true,
            sensitivity: 2000,
            ..Default::default()
        };
        let res = evaluate_motion_transition(v2, v1, &cfg);
        assert!(res.is_some());
        if let Some(MotionAction::TriggerAlarm { displacement, .. }) = res {
            assert_eq!(displacement, 5000);
        }
    }
}
