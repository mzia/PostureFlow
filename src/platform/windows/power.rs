use std::process::Command;

// Standard Windows Power Scheme GUIDs
pub const SCHEME_MAX_PERF: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c"; // High Performance
pub const SCHEME_BALANCED: &str = "381b4222-f694-41f0-9685-ff5bb260df2e"; // Balanced
pub const SCHEME_MIN_POWER: &str = "a1841308-3541-4fab-bc81-f71556f20b4a"; // Power Saver

pub fn apply_windows_power_profile(profile_id: &str) -> Result<(), String> {
    let scheme_guid = match profile_id {
        "dev" => SCHEME_MAX_PERF,
        "travel" => SCHEME_MIN_POWER,
        _ => SCHEME_BALANCED,
    };

    let output = Command::new("powercfg")
        .args(["/setactive", scheme_guid])
        .output()
        .map_err(|e| format!("Failed to set power scheme: {}", e))?;

    if !output.status.success() {
        return Err(format!("powercfg failed to set scheme {}", scheme_guid));
    }
    Ok(())
}

pub fn set_windows_idle_timeout(seconds: u32) {
    let minutes = (seconds / 60).max(1);
    let m_str = minutes.to_string();
    let _ = Command::new("powercfg").args(["/change", "monitor-timeout-ac", &m_str]).output();
    let _ = Command::new("powercfg").args(["/change", "monitor-timeout-dc", &m_str]).output();
    let _ = Command::new("powercfg").args(["/change", "standby-timeout-ac", &(minutes * 2).to_string()]).output();
    let _ = Command::new("powercfg").args(["/change", "standby-timeout-dc", &m_str]).output();
}
