use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PulsarKeyStatus {
    pub installed: bool,
    pub config_found: bool,
    pub profile: Option<String>,
    pub autolock: bool,
    pub sentinel_running: bool,
    pub applet_running: bool,
    pub sudo_pam_enforced: bool,
    pub greeter_pam_enforced: bool,
    pub polkit_pam_enforced: bool,
    pub registered_tokens_count: usize,
    pub has_user_verification: bool,
    pub fido2_ssh_keys: Vec<String>,
    pub summary: String,
}

impl Default for PulsarKeyStatus {
    fn default() -> Self {
        Self {
            installed: false,
            config_found: false,
            profile: None,
            autolock: false,
            sentinel_running: false,
            applet_running: false,
            sudo_pam_enforced: false,
            greeter_pam_enforced: false,
            polkit_pam_enforced: false,
            registered_tokens_count: 0,
            has_user_verification: false,
            fido2_ssh_keys: Vec::new(),
            summary: "Not Detected".to_string(),
        }
    }
}

#[derive(Deserialize)]
struct RawPulsarConfig {
    autolock: Option<bool>,
    profile: Option<String>,
}

/// Resolves user configuration path for PulsarKey (~/.config/pulsarkey/config.json)
pub fn get_pulsarkey_config_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("SUDO_USER").map(|u| format!("/home/{}", u)))
        .unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(".config/pulsarkey/config.json")
}

/// Check if a process containing target name is running
fn is_process_running(proc_name: &str) -> bool {
    #[cfg(unix)]
    {
        // 1. Check Linux /proc directly
        let proc_path = Path::new("/proc");
        if proc_path.exists() {
            if let Ok(entries) = fs::read_dir(proc_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let comm_file = path.join("comm");
                        if let Ok(comm) = fs::read_to_string(&comm_file) {
                            if comm.trim().contains(proc_name) {
                                return true;
                            }
                        }
                        let cmd_file = path.join("cmdline");
                        if let Ok(cmd) = fs::read_to_string(&cmd_file) {
                            if cmd.contains(proc_name) {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        // 2. Fallback to pgrep -f
        if let Ok(output) = Command::new("pgrep").arg("-f").arg(proc_name).output() {
            if output.status.success() && !output.stdout.is_empty() {
                return true;
            }
        }
    }
    false
}

/// Check if a PAM configuration file enforces pam_u2f.so
fn check_pam_file(path: &str) -> bool {
    if let Ok(content) = fs::read_to_string(path) {
        content.lines().any(|l| {
            let trimmed = l.trim();
            !trimmed.starts_with('#') && trimmed.contains("pam_u2f.so")
        })
    } else {
        false
    }
}

/// Parse /etc/yubico/u2f_keys for enrolled tokens and user verification flags
pub fn parse_u2f_mapping_content(content: &str) -> (usize, bool) {
    let mut count = 0;
    let mut has_uv = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.contains("+verification") {
            has_uv = true;
        }
        // Each entry is user:token1:token2...
        let parts: Vec<&str> = trimmed.split(':').skip(1).collect();
        count += parts.len();
    }
    (count, has_uv)
}

/// Inspect /etc/yubico/u2f_keys
fn check_enrolled_tokens() -> (usize, bool) {
    let mapping_path = Path::new("/etc/yubico/u2f_keys");
    if let Ok(content) = fs::read_to_string(mapping_path) {
        parse_u2f_mapping_content(&content)
    } else {
        (0, false)
    }
}

/// Inspect user's ~/.ssh directory for FIDO2 hardware-backed resident and non-resident keys (_sk)
fn check_fido2_ssh_keys() -> Vec<String> {
    let mut found = Vec::new();
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("SUDO_USER").map(|u| format!("/home/{}", u)))
        .unwrap_or_else(|_| "/root".to_string());
    let ssh_dir = PathBuf::from(home).join(".ssh");
    if let Ok(entries) = fs::read_dir(ssh_dir) {
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.contains("_sk") && !fname.ends_with(".pub") {
                found.push(fname);
            }
        }
    }
    found.sort();
    found
}

/// Detect full PulsarKey installation, configuration, PAM, and Sentinel state
pub fn detect_pulsarkey() -> PulsarKeyStatus {
    let config_path = get_pulsarkey_config_path();
    let config_found = config_path.exists();

    let mut autolock = false;
    let mut profile = None;

    if config_found {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(parsed) = serde_json::from_str::<RawPulsarConfig>(&content) {
                autolock = parsed.autolock.unwrap_or(true);
                profile = parsed.profile;
            }
        }
    }

    // Process checks
    let sentinel_running = is_process_running("pulsarkey-sentinel") || is_process_running("pulsarkey sentinel");
    let applet_running = is_process_running("pulsarkey-applet") || is_process_running("pulsarkey applet");

    // PAM checks
    #[cfg(target_os = "macos")]
    let sudo_pam_enforced = check_pam_file("/etc/pam.d/sudo_local") || check_pam_file("/etc/pam.d/sudo");
    #[cfg(not(target_os = "macos"))]
    let sudo_pam_enforced = check_pam_file("/etc/pam.d/sudo");

    #[cfg(target_os = "macos")]
    let greeter_pam_enforced = check_pam_file("/etc/pam.d/screensaver");
    #[cfg(not(target_os = "macos"))]
    let greeter_pam_enforced = check_pam_file("/etc/pam.d/cosmic-greeter") || check_pam_file("/etc/pam.d/gdm-password");

    #[cfg(target_os = "macos")]
    let polkit_pam_enforced = check_pam_file("/etc/pam.d/authorization");
    #[cfg(not(target_os = "macos"))]
    let polkit_pam_enforced = check_pam_file("/etc/pam.d/polkit-1");

    // Enrolled tokens & FIDO2 SSH
    let (registered_tokens_count, has_user_verification) = check_enrolled_tokens();
    let fido2_ssh_keys = check_fido2_ssh_keys();

    // Binary / installation check
    let binary_exists = Path::new("/usr/bin/pulsarkey").exists()
        || Path::new("/usr/local/bin/pulsarkey").exists()
        || Path::new("/usr/bin/cosmic-fido2").exists()
        || Command::new("which")
            .arg("pulsarkey")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

    let installed = binary_exists
        || config_found
        || (registered_tokens_count > 0)
        || sudo_pam_enforced
        || sentinel_running
        || applet_running;

    let summary = if !installed && !config_found {
        "Not Installed".to_string()
    } else {
        let prof_str = match profile.as_deref() {
            Some("lockdown") => "Lockdown (Mandatory)",
            Some("fortress") => "Fortress (2FA)",
            Some("convenience") => "Convenience (1FA)",
            Some(other) => other,
            None => "Configured",
        };

        let sentinel_tag = if sentinel_running {
            "Sentinel: Armed"
        } else if autolock {
            "Sentinel: Standby"
        } else {
            "Sentinel: Off"
        };

        format!("{} • {}", prof_str, sentinel_tag)
    };

    PulsarKeyStatus {
        installed,
        config_found,
        profile,
        autolock,
        sentinel_running,
        applet_running,
        sudo_pam_enforced,
        greeter_pam_enforced,
        polkit_pam_enforced,
        registered_tokens_count,
        has_user_verification,
        fido2_ssh_keys,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pulsarkey_default_status() {
        let status = PulsarKeyStatus::default();
        assert!(!status.installed);
        assert_eq!(status.summary, "Not Detected");
        assert_eq!(status.registered_tokens_count, 0);
    }

    #[test]
    fn test_parse_u2f_mapping_content() {
        let mapping = "mzia:token_handle_abc,pubkey1:token_handle_def,pubkey2+verification\n# comment\n";
        let (count, has_uv) = parse_u2f_mapping_content(mapping);
        assert_eq!(count, 2);
        assert!(has_uv);
    }

    #[test]
    fn test_raw_config_parsing() {
        let json = r#"{"autolock": true, "profile": "fortress"}"#;
        let parsed: RawPulsarConfig = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.autolock, Some(true));
        assert_eq!(parsed.profile.as_deref(), Some("fortress"));
    }

    #[test]
    fn test_pulsarkey_serialization_roundtrip() {
        let status = PulsarKeyStatus {
            installed: true,
            config_found: true,
            profile: Some("lockdown".to_string()),
            autolock: true,
            sentinel_running: true,
            applet_running: false,
            sudo_pam_enforced: true,
            greeter_pam_enforced: true,
            polkit_pam_enforced: true,
            registered_tokens_count: 2,
            has_user_verification: true,
            fido2_ssh_keys: vec!["id_ed25519_sk".to_string()],
            summary: "Lockdown (Mandatory) • Sentinel: Armed".to_string(),
        };

        let serialized = serde_json::to_string(&status).unwrap();
        let deserialized: PulsarKeyStatus = serde_json::from_str(&serialized).unwrap();
        assert_eq!(status, deserialized);
    }

    #[test]
    fn test_empty_u2f_mapping() {
        let (count, has_uv) = parse_u2f_mapping_content("");
        assert_eq!(count, 0);
        assert!(!has_uv);

        let (count, has_uv) = parse_u2f_mapping_content("# Just comments\n# Another comment\n");
        assert_eq!(count, 0);
        assert!(!has_uv);
    }

    #[test]
    fn test_summary_formatting() {
        let mut status = PulsarKeyStatus::default();
        assert_eq!(status.summary, "Not Detected");

        status.installed = true;
        status.profile = Some("fortress".to_string());
        status.sentinel_running = true;
        assert_eq!(status.profile.as_deref(), Some("fortress"));
    }
}
