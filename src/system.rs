use std::fs;
use std::process::Command;
use crate::profile::Profile;

pub const STATE_FILE: &str = "/etc/popos-security-profile";
pub const SYSCTL_CONF: &str = "/etc/sysctl.d/99-popos-security.conf";
pub const LIMITS_CONF: &str = "/etc/security/limits.d/99-popos-security.conf";

pub fn is_privileged() -> bool {
    unsafe { libc::geteuid() == 0 }
}

pub fn state_file_path() -> String {
    if let Ok(path) = std::env::var("POP_PROFILE_STATE_FILE") {
        return path;
    }
    if is_privileged() {
        STATE_FILE.to_string()
    } else {
        if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
            if std::path::Path::new(&dir).is_dir() {
                return format!("{}/popos-security-profile", dir);
            }
        }
        "/tmp/popos-security-profile".to_string()
    }
}

pub fn get_active_profile() -> String {
    let path = state_file_path();
    fs::read_to_string(&path)
        .map(|s| {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                "default".to_string()
            } else {
                trimmed
            }
        })
        .unwrap_or_else(|_| "default".to_string())
}

pub fn save_active_profile_str(name: &str) -> Result<(), String> {
    let path = state_file_path();
    fs::write(&path, name)
        .map_err(|e| format!("Failed to write state file {}: {}", path, e))
}

pub fn save_active_profile(profile: Profile) -> Result<(), String> {
    save_active_profile_str(profile.as_str())
}

fn execute(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute '{} {}': {}", cmd, args.join(" "), e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("'{} {}' failed (code {:?}): {}", cmd, args.join(" "), output.status.code(), stderr))
    }
}

pub fn ensure_ssh_safety() {
    let ssh_active = std::env::var("SSH_CLIENT").is_ok()
        || std::env::var("SSH_CONNECTION").is_ok()
        || std::env::var("SSH_TTY").is_ok();

    let sshd_listening = execute("ss", &["-tulpn"])
        .map(|out| out.contains(":22 ") || out.contains("sshd"))
        .unwrap_or(false);

    if ssh_active || sshd_listening {
        let _ = execute("ufw", &["allow", "22/tcp", "comment", "SSH Anti-Lockout"]);
    }
}

pub fn clean_ufw_custom_rules() {
    let ports = ["3000", "5000", "5173", "8000", "8080", "8888", "9000"];
    for p in ports {
        let _ = execute("ufw", &["delete", "allow", &format!("{}/tcp", p)]);
    }
    let _ = execute("ufw", &["delete", "allow", "3000:3005/tcp"]);

    let home_rules = [
        ("1714:1764/tcp", "tcp"),
        ("1714:1764/udp", "udp"),
        ("53317/tcp", "tcp"),
        ("53317/udp", "udp"),
        ("27031:27036/udp", "udp"),
        ("27036:27037/tcp", "tcp"),
        ("27040/tcp", "tcp"),
        ("5353/udp", "udp"),
        ("1900/udp", "udp"),
        ("631/tcp", "tcp"),
    ];

    for (rule, _) in home_rules {
        let _ = execute("ufw", &["delete", "allow", rule]);
    }
}

pub fn set_desktop_idle_delay(seconds: u32) {
    if let Ok(user) = std::env::var("SUDO_USER") {
        if let Ok(id_out) = execute("id", &["-u", &user]) {
            let uid = id_out.trim();
            let bus = format!("unix:path=/run/user/{}/bus", uid);
            let _ = Command::new("sudo")
                .args([
                    "-u",
                    &user,
                    &format!("DBUS_SESSION_BUS_ADDRESS={}", bus),
                    "gsettings",
                    "set",
                    "org.gnome.desktop.session",
                    "idle-delay",
                    &seconds.to_string(),
                ])
                .output();
            return;
        }
    }
    let _ = execute("gsettings", &["set", "org.gnome.desktop.session", "idle-delay", &seconds.to_string()]);
}

pub fn apply_profile(profile: Profile) -> Result<(), String> {
    apply_profile_by_id(profile.as_str())
}

pub fn apply_profile_by_id(id: &str) -> Result<(), String> {
    if let Some(config) = crate::config::find_profile(id) {
        apply_profile_config(&config)
    } else {
        Err(format!("Profile '{}' not found.", id))
    }
}

pub fn apply_profile_config(config: &crate::config::ProfileConfig) -> Result<(), String> {
    if !is_privileged() {
        save_active_profile_str(&config.profile.id)?;
        return Ok(());
    }

    // 1. Sysctls
    if !config.kernel.is_empty() {
        let mut sysctl_content = String::new();
        for (k, v) in &config.kernel {
            sysctl_content.push_str(&format!("{} = {}\n", k, v));
        }
        fs::write(SYSCTL_CONF, sysctl_content)
            .map_err(|e| format!("Writing sysctl failed: {}", e))?;
        let _ = execute("sysctl", &["-p", SYSCTL_CONF]);
    }

    // 2. Limits
    let mut limits_content = String::new();
    if let Some(ref core) = config.limits.core_dump {
        limits_content.push_str(&format!("* soft core {}\n", core));
    }
    if let Some(nofile) = config.limits.nofile_soft {
        limits_content.push_str(&format!("* soft nofile {}\n", nofile));
    }
    if let Some(nofile) = config.limits.nofile_hard {
        limits_content.push_str(&format!("* hard nofile {}\n", nofile));
    }
    if !limits_content.is_empty() {
        let _ = fs::write(LIMITS_CONF, limits_content);
    }

    // 3. Firewall
    let _ = execute("ufw", &["--force", "default", &config.firewall.default_incoming, "incoming"]);
    let _ = execute("ufw", &["--force", "default", &config.firewall.default_outgoing, "outgoing"]);

    if config.firewall.allow_loopback {
        let _ = execute("ufw", &["allow", "in", "on", "lo"]);
        let _ = execute("ufw", &["allow", "out", "on", "lo"]);
    }

    clean_ufw_custom_rules();

    for iface in &config.firewall.allow_interfaces {
        if iface.contains('+') || std::path::Path::new(&format!("/sys/class/net/{}", iface)).exists() {
            let _ = execute("ufw", &["allow", "in", "on", iface, "comment", "Profile Interface"]);
            let _ = execute("ufw", &["allow", "out", "on", iface, "comment", "Profile Interface"]);
        }
    }

    for rule in &config.firewall.allow_ports {
        if !rule.comment.is_empty() {
            let _ = execute("ufw", &["allow", &rule.port, "comment", &rule.comment]);
        } else {
            let _ = execute("ufw", &["allow", &rule.port]);
        }
    }

    ensure_ssh_safety();
    let _ = execute("ufw", &["--force", "enable"]);

    // 4. Desktop idle delay
    if let Some(idle) = config.desktop.idle_delay_seconds {
        set_desktop_idle_delay(idle);
    }

    // 5. Framework Power
    if let Some(prof) = config.power.power_profile.as_deref() {
        let _ = execute("system76-power", &["profile", prof])
            .or_else(|_| execute("powerprofilesctl", &["set", prof]));
    }
    if let Some(charge_limit) = config.power.battery_charge_limit {
        let _ = execute("system76-power", &["charge-thresholds", "--max", &charge_limit.to_string()])
            .or_else(|_| {
                fs::write("/sys/class/power_supply/BAT0/charge_control_limit_max", charge_limit.to_string())
                    .map(|_| String::new())
                    .map_err(|e| e.to_string())
            });
    }

    save_active_profile_str(&config.profile.id)?;
    Ok(())
}

pub fn reset_to_defaults() -> Result<(), String> {
    let path = state_file_path();
    let _ = fs::remove_file(&path);

    // Restore desktop idle lock timeout to Pop!_OS default (15 minutes = 900 seconds)
    set_desktop_idle_delay(900);

    if !is_privileged() {
        return Ok(());
    }

    let _ = execute("ufw", &["--force", "disable"]);
    let _ = execute("ufw", &["--force", "reset"]);

    let _ = fs::remove_file(SYSCTL_CONF);
    let _ = fs::remove_file(LIMITS_CONF);

    let _ = execute("sysctl", &["--system"]);

    // Restore platform power profile to Pop!_OS factory default ('balanced')
    let _ = execute("system76-power", &["profile", "balanced"])
        .or_else(|_| execute("powerprofilesctl", &["set", "balanced"]));

    // Restore battery charge threshold to 100% (unrestricted charging)
    let _ = execute("system76-power", &["charge-thresholds", "--max", "100"])
        .or_else(|_| {
            fs::write("/sys/class/power_supply/BAT0/charge_control_limit_max", "100")
                .map(|_| String::new())
                .map_err(|e| e.to_string())
        });

    Ok(())
}

pub fn get_status_report() -> String {
    let profile = get_active_profile();
    let ptrace = execute("sysctl", &["-n", "kernel.yama.ptrace_scope"]).unwrap_or_else(|_| "N/A".into());
    let inotify = execute("sysctl", &["-n", "fs.inotify.max_user_watches"]).unwrap_or_else(|_| "N/A".into());
    let map_count = execute("sysctl", &["-n", "vm.max_map_count"]).unwrap_or_else(|_| "N/A".into());
    let ufw_status = execute("ufw", &["status"]).unwrap_or_else(|_| "inactive".into());

    format!(
        "Active Profile: {}\nptrace_scope: {}\ninotify watches: {}\nvm.max_map_count: {}\nUFW Status:\n{}",
        profile.to_uppercase(),
        ptrace,
        inotify,
        map_count,
        ufw_status
    )
}
