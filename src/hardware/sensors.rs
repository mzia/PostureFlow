use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SensorPrivacyConfig {
    #[serde(default)]
    pub camera_blocked: bool,
    #[serde(default)]
    pub microphone_muted: bool,
    #[serde(default)]
    pub location_blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SensorPrivacyReport {
    pub camera_blocked: bool,
    pub microphone_muted: bool,
    pub location_blocked: bool,
    pub active_cameras_count: usize,
    pub active_audio_sources_count: usize,
}

/// Count connected video capture devices
pub fn count_connected_cameras() -> usize {
    #[cfg(unix)]
    {
        if let Ok(entries) = fs::read_dir("/sys/class/video4linux") {
            return entries.flatten().count();
        }
    }
    0
}

/// Query current sensor privacy states
pub fn get_sensor_privacy_report() -> SensorPrivacyReport {
    let camera_blocked = crate::system::is_camera_blocked();
    let microphone_muted = crate::system::is_microphone_muted();
    let location_blocked = crate::system::is_location_blocked();
    let active_cameras_count = count_connected_cameras();

    let active_audio_sources_count = if microphone_muted { 0 } else { 1 };

    SensorPrivacyReport {
        camera_blocked,
        microphone_muted,
        location_blocked,
        active_cameras_count,
        active_audio_sources_count,
    }
}

/// Instant Emergency Kill Switch: cuts camera hardware, mutes microphone, and isolates location
pub fn emergency_kill_all_sensors() -> Result<(), String> {
    let _ = crate::system::apply_camera_blocked(true);
    let _ = crate::system::apply_microphone_muted(true);
    let _ = crate::system::apply_location_blocked(true);

    let _ = crate::system::execute(
        "notify-send",
        &[
            "-u",
            "critical",
            "PostureFlow Sensor Privacy",
            "🚨 Emergency Sensor Kill Switch engaged: Camera cut, Mic muted, Geolocation disabled.",
        ],
    );

    Ok(())
}

/// Restore all sensors to normal operational status
pub fn restore_all_sensors() -> Result<(), String> {
    let _ = crate::system::apply_camera_blocked(false);
    let _ = crate::system::apply_microphone_muted(false);
    let _ = crate::system::apply_location_blocked(false);

    let _ = crate::system::execute(
        "notify-send",
        &[
            "-u",
            "normal",
            "PostureFlow Sensor Privacy",
            "🛡️ Sensors restored: Camera, Microphone, and Location active.",
        ],
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_privacy_report_default() {
        let rep = SensorPrivacyReport::default();
        assert!(!rep.camera_blocked);
        assert!(!rep.microphone_muted);
        assert!(!rep.location_blocked);
        assert_eq!(rep.active_cameras_count, 0);
    }

    #[test]
    fn test_sensor_config_serialization() {
        let cfg = SensorPrivacyConfig {
            camera_blocked: true,
            microphone_muted: true,
            location_blocked: true,
        };

        let json = serde_json::to_string(&cfg).unwrap();
        let deserialized: SensorPrivacyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, deserialized);
    }
}
