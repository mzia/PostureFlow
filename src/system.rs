use std::fs;
use std::process::Command;
use serde::{Deserialize, Serialize};
use crate::profile::Profile;

#[cfg(unix)]
pub const STATE_FILE: &str = "/etc/postureflow-state";
#[cfg(windows)]
pub const STATE_FILE: &str = r"C:\ProgramData\PostureFlow\state";

pub const SYSCTL_CONF: &str = "/etc/sysctl.d/99-postureflow.conf";
pub const LIMITS_CONF: &str = "/etc/security/limits.d/99-postureflow.conf";
pub const RESOLVED_CONF: &str = "/etc/systemd/resolved.conf.d/99-postureflow.conf";

#[cfg(unix)]
pub fn is_privileged() -> bool {
    unsafe { libc::geteuid() == 0 }
}

#[cfg(windows)]
pub fn is_privileged() -> bool {
    // In Windows, elevated administrator/SYSTEM processes succeed on `net session`
    std::process::Command::new("net")
        .arg("session")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn state_file_path() -> String {
    if let Ok(path) = std::env::var("POSTUREFLOW_STATE_FILE") {
        return path;
    }
    #[cfg(windows)]
    {
        if is_privileged() {
            if let Ok(pd) = std::env::var("ProgramData") {
                let dir = format!(r"{}\PostureFlow", pd);
                let _ = fs::create_dir_all(&dir);
                return format!(r"{}\state", dir);
            }
            STATE_FILE.to_string()
        } else {
            if let Ok(la) = std::env::var("LOCALAPPDATA") {
                let dir = format!(r"{}\PostureFlow", la);
                let _ = fs::create_dir_all(&dir);
                return format!(r"{}\state", dir);
            }
            r"C:\Temp\postureflow-state".to_string()
        }
    }
    #[cfg(unix)]
    {
        if is_privileged() {
            STATE_FILE.to_string()
        } else {
            if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
                if std::path::Path::new(&dir).is_dir() {
                    return format!("{}/postureflow-state", dir);
                }
            }
            "/tmp/postureflow-state".to_string()
        }
    }
}

pub fn get_active_profile() -> String {
    // 1. Primary source of truth: /etc/postureflow-state (system-wide, world-readable)
    if let Ok(s) = fs::read_to_string(STATE_FILE) {
        let trimmed = s.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }

    // 2. Fallback to state_file_path()
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
    if is_privileged() {
        let _ = fs::write(STATE_FILE, name);
    }
    let path = state_file_path();
    if path != STATE_FILE {
        let _ = fs::write(&path, name);
    }
    Ok(())
}

pub fn save_active_profile(profile: Profile) -> Result<(), String> {
    save_active_profile_str(profile.as_str())
}


pub fn execute(cmd: &str, args: &[&str]) -> Result<String, String> {
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
    let previous_id = get_active_profile();
    let previous_config = crate::config::find_profile(&previous_id);

    if !is_privileged() {
        apply_lifecycle_hooks(previous_config.as_ref(), config);
        save_active_profile_str(&config.profile.id)?;
        return Ok(());
    }

    #[cfg(windows)]
    {
        crate::platform::windows::firewall::apply_windows_firewall(config)?;
        let _ = crate::platform::windows::power::apply_windows_power_profile(&config.profile.id);
        if let Some(idle) = config.desktop.idle_delay_seconds {
            crate::platform::windows::power::set_windows_idle_timeout(idle);
        }
        apply_lifecycle_hooks(previous_config.as_ref(), config);
        save_active_profile_str(&config.profile.id)?;
        return Ok(());
    }

    #[cfg(unix)]
    {
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

    // 6. Deep Hardware & Display Power Orchestration
    if let Some(ref epp) = config.power.cpu_epp {
        let _ = apply_cpu_epp(epp);
    }
    if let Some(block_new_usb) = config.peripherals.block_new_usb {
        let _ = apply_usb_lockdown(block_new_usb);
    }
    if let Some(bt) = config.peripherals.bluetooth {
        let _ = apply_bluetooth(bt);
    }
    if let Some(cam) = config.peripherals.camera_blocked {
        let _ = apply_camera_blocked(cam);
    }
    if let Some(mic) = config.peripherals.microphone_muted {
        let _ = apply_microphone_muted(mic);
    }
    if let Some(loc) = config.peripherals.location_blocked {
        let _ = apply_location_blocked(loc);
    }

    // 7. Profile-Aware Encrypted DNS & Privacy Orchestration
    let _ = apply_dns_config(&config.dns);

    apply_lifecycle_hooks(previous_config.as_ref(), config);
    save_active_profile_str(&config.profile.id)?;
    Ok(())
    }
}

pub fn apply_lifecycle_hooks(previous: Option<&crate::config::ProfileConfig>, target: &crate::config::ProfileConfig) {
    #[cfg(unix)]
    {
        // 1. Exiting previous profile: run on_exit command and pause dev services
        if let Some(prev) = previous {
            if prev.profile.id != target.profile.id {
                if let Some(ref exit_cmd) = prev.hooks.on_exit {
                    let _ = Command::new("sh").args(["-c", exit_cmd]).spawn();
                }

                // If leaving Docker-managed profile (e.g. dev) and target does not manage it, pause containers
                if prev.hooks.manage_docker == Some(true) && target.hooks.manage_docker != Some(true) {
                    let _ = Command::new("sh")
                        .args(["-c", "if command -v docker >/dev/null 2>&1; then c=$(docker ps -q 2>/dev/null); if [ -n \"$c\" ]; then docker pause $c >/dev/null 2>&1; fi; fi"])
                        .spawn();
                }

                // Stop user systemd services managed by previous profile if not in target
                for srv in &prev.hooks.manage_systemd_services {
                    if !target.hooks.manage_systemd_services.contains(srv) {
                        let _ = Command::new("systemctl")
                            .args(["--user", "stop", srv])
                            .spawn();
                    }
                }
            }
        }

        // 2. Entering target profile: resume Docker containers & start user services
        if target.hooks.manage_docker == Some(true) {
            let _ = Command::new("sh")
                .args(["-c", "if command -v docker >/dev/null 2>&1; then p=$(docker ps -q -f status=paused 2>/dev/null); if [ -n \"$p\" ]; then docker unpause $p >/dev/null 2>&1; fi; fi"])
                .spawn();
        }

        for srv in &target.hooks.manage_systemd_services {
            let _ = Command::new("systemctl")
                .args(["--user", "start", srv])
                .spawn();
        }

        if let Some(ref enter_cmd) = target.hooks.on_enter {
            let _ = Command::new("sh").args(["-c", enter_cmd]).spawn();
        }
    }
}

pub fn apply_cpu_epp(preference: &str) -> Result<(), String> {
    if let Ok(entries) = fs::read_dir("/sys/devices/system/cpu/cpufreq") {
        for entry in entries.flatten() {
            let path = entry.path().join("energy_performance_preference");
            if path.exists() {
                let _ = fs::write(&path, preference.trim());
            }
        }
    }
    Ok(())
}

pub fn get_cpu_epp() -> Option<String> {
    if let Ok(val) = fs::read_to_string("/sys/devices/system/cpu/cpufreq/policy0/energy_performance_preference") {
        let trimmed = val.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    None
}

pub fn apply_usb_lockdown(block_new: bool) -> Result<(), String> {
    let val = if block_new { "0" } else { "1" };
    if let Ok(entries) = fs::read_dir("/sys/bus/usb/devices") {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.starts_with("usb") {
                let path = entry.path().join("authorized_default");
                if path.exists() {
                    let _ = fs::write(&path, val);
                }
            }
        }
    }
    Ok(())
}

pub fn is_usb_locked_down() -> bool {
    if let Ok(entries) = fs::read_dir("/sys/bus/usb/devices") {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.starts_with("usb") {
                let path = entry.path().join("authorized_default");
                if let Ok(val) = fs::read_to_string(&path) {
                    if val.trim() == "0" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub fn apply_bluetooth(enabled: bool) -> Result<(), String> {
    if enabled {
        let _ = execute("rfkill", &["unblock", "bluetooth"]);
    } else {
        let _ = execute("rfkill", &["block", "bluetooth"]);
    }
    Ok(())
}

pub fn is_bluetooth_blocked() -> bool {
    execute("rfkill", &["list", "bluetooth"])
        .map(|out| out.contains("Soft blocked: yes") || out.contains("Hard blocked: yes"))
        .unwrap_or(false)
}

pub fn lock_desktop_session() -> Result<(), String> {
    #[cfg(unix)]
    {
        let _ = execute("loginctl", &["lock-session"]);
        let _ = execute("xdg-screensaver", &["lock"]);
        let _ = execute("gnome-screensaver-command", &["-l"]);
        Ok(())
    }
    #[cfg(windows)]
    {
        let _ = Command::new("rundll32.exe")
            .args(["user32.dll,LockWorkStation"])
            .output();
        Ok(())
    }
}

pub fn apply_camera_blocked(blocked: bool) -> Result<(), String> {
    #[cfg(unix)]
    {
        let flag_path = std::path::Path::new("/run/postureflow_camera_blocked");
        let fallback_flag = std::path::Path::new("/tmp/postureflow_camera_blocked");
        if blocked {
            let _ = fs::write(flag_path, "blocked");
            let _ = fs::write(fallback_flag, "blocked");

            // Attempt kernel driver unbind if privileged
            if is_privileged() {
                if let Ok(entries) = fs::read_dir("/sys/bus/usb/drivers/uvcvideo") {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.contains(':') {
                            let _ = fs::write("/sys/bus/usb/drivers/uvcvideo/unbind", &name);
                        }
                    }
                }
            }
        } else {
            let _ = fs::remove_file(flag_path);
            let _ = fs::remove_file(fallback_flag);

            // Re-bind devices or reload driver
            if is_privileged() {
                let _ = execute("modprobe", &["uvcvideo"]);
            }
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        let _ = blocked;
        Ok(())
    }
}

pub fn is_camera_blocked() -> bool {
    #[cfg(unix)]
    {
        std::path::Path::new("/run/postureflow_camera_blocked").exists()
            || std::path::Path::new("/tmp/postureflow_camera_blocked").exists()
    }
    #[cfg(windows)]
    {
        false
    }
}

pub fn apply_microphone_muted(muted: bool) -> Result<(), String> {
    #[cfg(unix)]
    {
        let flag_path = std::path::Path::new("/run/postureflow_mic_muted");
        let fallback_flag = std::path::Path::new("/tmp/postureflow_mic_muted");
        if muted {
            let _ = fs::write(flag_path, "muted");
            let _ = fs::write(fallback_flag, "muted");
            let _ = execute("wpctl", &["set-mute", "@DEFAULT_AUDIO_SOURCE@", "1"]);
            let _ = execute("pactl", &["set-source-mute", "@DEFAULT_SOURCE@", "1"]);
            let _ = execute("amixer", &["set", "Capture", "nocap"]);
        } else {
            let _ = fs::remove_file(flag_path);
            let _ = fs::remove_file(fallback_flag);
            let _ = execute("wpctl", &["set-mute", "@DEFAULT_AUDIO_SOURCE@", "0"]);
            let _ = execute("pactl", &["set-source-mute", "@DEFAULT_SOURCE@", "0"]);
            let _ = execute("amixer", &["set", "Capture", "cap"]);
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        let _ = muted;
        Ok(())
    }
}

pub fn is_microphone_muted() -> bool {
    #[cfg(unix)]
    {
        if std::path::Path::new("/run/postureflow_mic_muted").exists()
            || std::path::Path::new("/tmp/postureflow_mic_muted").exists()
        {
            return true;
        }
        if let Ok(out) = execute("wpctl", &["get-volume", "@DEFAULT_AUDIO_SOURCE@"]) {
            if out.contains("[MUTED]") {
                return true;
            }
        }
        if let Ok(out) = execute("pactl", &["get-source-mute", "@DEFAULT_SOURCE@"]) {
            if out.to_lowercase().contains("yes") {
                return true;
            }
        }
        false
    }
    #[cfg(windows)]
    {
        false
    }
}

pub fn apply_location_blocked(blocked: bool) -> Result<(), String> {
    #[cfg(unix)]
    {
        let flag_path = std::path::Path::new("/run/postureflow_loc_blocked");
        let fallback_flag = std::path::Path::new("/tmp/postureflow_loc_blocked");
        if blocked {
            let _ = fs::write(flag_path, "blocked");
            let _ = fs::write(fallback_flag, "blocked");
            let _ = execute("systemctl", &["stop", "geoclue.service"]);
            let _ = execute("gsettings", &["set", "org.gnome.system.location", "enabled", "false"]);
        } else {
            let _ = fs::remove_file(flag_path);
            let _ = fs::remove_file(fallback_flag);
            let _ = execute("systemctl", &["start", "geoclue.service"]);
            let _ = execute("gsettings", &["set", "org.gnome.system.location", "enabled", "true"]);
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        let _ = blocked;
        Ok(())
    }
}

pub fn is_location_blocked() -> bool {
    #[cfg(unix)]
    {
        std::path::Path::new("/run/postureflow_loc_blocked").exists()
            || std::path::Path::new("/tmp/postureflow_loc_blocked").exists()
    }
    #[cfg(windows)]
    {
        false
    }
}

pub fn block_offender_ip(ip: &str) -> Result<(), String> {
    #[cfg(unix)]
    {
        if !is_privileged() {
            return Err("Root/Administrator privileges required to block IP address".to_string());
        }
        execute("ufw", &["insert", "1", "deny", "from", ip, "comment", "PostureFlow Honeypot Trap"])?;
        let _ = execute("ufw", &["reload"]);
        Ok(())
    }
    #[cfg(windows)]
    {
        let rule_name = format!("PostureFlow-Honeypot-{}", ip);
        let _ = Command::new("netsh")
            .args([
                "advfirewall", "firewall", "add", "rule",
                &format!("name={}", rule_name),
                "dir=in", "action=block",
                &format!("remoteip={}", ip),
                "enable=yes",
                "description=Blocked via PostureFlow Honeypot Trap",
            ])
            .output();
        Ok(())
    }
}

pub fn apply_dns_config(dns: &crate::config::schema::DnsConfig) -> Result<(), String> {
    #[cfg(unix)]
    {
        if !is_privileged() {
            return Ok(());
        }

        let has_custom_dns = !dns.servers.is_empty()
            || !dns.fallback_servers.is_empty()
            || dns.dns_over_tls.is_some()
            || dns.dnssec.is_some()
            || !dns.domains.is_empty();

        if !has_custom_dns {
            if std::path::Path::new(RESOLVED_CONF).exists() {
                let _ = fs::remove_file(RESOLVED_CONF);
                let _ = execute("systemctl", &["reload-or-restart", "systemd-resolved"]);
            }
            return Ok(());
        }

        let mut conf = String::from("# PostureFlow Profile-Aware Encrypted DNS & Privacy Configuration\n[Resolve]\n");
        if !dns.servers.is_empty() {
            conf.push_str(&format!("DNS={}\n", dns.servers.join(" ")));
        }
        if !dns.fallback_servers.is_empty() {
            conf.push_str(&format!("FallbackDNS={}\n", dns.fallback_servers.join(" ")));
        }
        if let Some(ref dot) = dns.dns_over_tls {
            conf.push_str(&format!("DNSOverTLS={}\n", dot));
        }
        if let Some(ref sec) = dns.dnssec {
            conf.push_str(&format!("DNSSEC={}\n", sec));
        }
        if !dns.domains.is_empty() {
            conf.push_str(&format!("Domains={}\n", dns.domains.join(" ")));
        }

        if let Some(parent) = std::path::Path::new(RESOLVED_CONF).parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(RESOLVED_CONF, conf).map_err(|e| format!("Failed to write resolved drop-in: {}", e))?;
        let _ = execute("systemctl", &["reload-or-restart", "systemd-resolved"]);
    }
    Ok(())
}

pub fn clear_dns_config() -> Result<(), String> {
    #[cfg(unix)]
    {
        if !is_privileged() {
            return Ok(());
        }
        if std::path::Path::new(RESOLVED_CONF).exists() {
            let _ = fs::remove_file(RESOLVED_CONF);
            let _ = execute("systemctl", &["reload-or-restart", "systemd-resolved"]);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DnsStatusReport {
    pub is_encrypted: bool,
    pub dns_over_tls: String,
    pub dnssec: String,
    pub active_servers: Vec<String>,
    pub managed_by_postureflow: bool,
}

pub fn get_dns_status() -> DnsStatusReport {
    #[cfg(unix)]
    {
        let managed = std::path::Path::new(RESOLVED_CONF).exists();
        let mut is_encrypted = false;
        let mut dns_over_tls = "no".to_string();
        let mut dnssec = "no".to_string();
        let mut active_servers = Vec::new();

        if let Ok(content) = fs::read_to_string(RESOLVED_CONF) {
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("DNS=") {
                    active_servers = rest.split_whitespace().map(|s| s.to_string()).collect();
                } else if let Some(rest) = trimmed.strip_prefix("DNSOverTLS=") {
                    dns_over_tls = rest.to_string();
                    if dns_over_tls == "yes" || dns_over_tls == "opportunistic" {
                        is_encrypted = true;
                    }
                } else if let Some(rest) = trimmed.strip_prefix("DNSSEC=") {
                    dnssec = rest.to_string();
                }
            }
        }

        if active_servers.is_empty() {
            if let Ok(out) = execute("resolvectl", &["status", "--no-pager"]) {
                if out.contains("+DNSOverTLS") || out.contains("DNSOverTLS=yes") {
                    is_encrypted = true;
                    dns_over_tls = "yes".to_string();
                }
                if out.contains("DNSSEC=yes") {
                    dnssec = "yes".to_string();
                }
                for line in out.lines() {
                    if line.contains("DNS Servers:") || line.contains("Current DNS Server:") {
                        let parts: Vec<&str> = line.split(':').collect();
                        if parts.len() >= 2 {
                            for s in parts[1].split_whitespace() {
                                if !active_servers.contains(&s.to_string()) {
                                    active_servers.push(s.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        DnsStatusReport {
            is_encrypted,
            dns_over_tls,
            dnssec,
            active_servers,
            managed_by_postureflow: managed,
        }
    }
    #[cfg(windows)]
    {
        DnsStatusReport {
            is_encrypted: false,
            dns_over_tls: "no".to_string(),
            dnssec: "no".to_string(),
            active_servers: Vec::new(),
            managed_by_postureflow: false,
        }
    }
}

pub fn reset_to_defaults() -> Result<(), String> {
    let path = state_file_path();
    let _ = fs::remove_file(&path);

    #[cfg(windows)]
    {
        crate::platform::windows::power::set_windows_idle_timeout(900);
        if !is_privileged() {
            return Ok(());
        }
        crate::platform::windows::firewall::clean_postureflow_firewall_rules();
        let _ = crate::platform::windows::power::apply_windows_power_profile("home");
        let _ = fs::remove_file(STATE_FILE);
        return Ok(());
    }

    #[cfg(unix)]
    {
        // Restore desktop idle lock timeout to Pop!_OS default (15 minutes = 900 seconds)
        set_desktop_idle_delay(900);

        if !is_privileged() {
            return Ok(());
        }

        let _ = execute("ufw", &["--force", "disable"]);
        let _ = execute("ufw", &["--force", "reset"]);

        let _ = fs::remove_file(STATE_FILE);
        let _ = fs::remove_file(SYSCTL_CONF);
        let _ = fs::remove_file(LIMITS_CONF);
        let _ = fs::remove_file("/etc/popos-security-profile");
        let _ = fs::remove_file("/etc/sysctl.d/99-popos-security.conf");
        let _ = fs::remove_file("/etc/security/limits.d/99-popos-security.conf");

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

        // Restore CPU EPP to factory balanced performance
        let _ = apply_cpu_epp("balance_performance");

        // Restore USB authorization to standard auto-authorize
        let _ = apply_usb_lockdown(false);

        // Restore Bluetooth radio to unblocked
        let _ = apply_bluetooth(true);

        // Clear encrypted DNS configuration and restore default resolution
        let _ = clear_dns_config();

        // Restore hardware sensors to unblocked / unmuted
        let _ = apply_camera_blocked(false);
        let _ = apply_microphone_muted(false);
        let _ = apply_location_blocked(false);

        Ok(())
    }
}

pub fn get_status_report() -> String {
    let profile = get_active_profile();

    #[cfg(windows)]
    {
        format!(
            "Active Profile: {}\nPlatform:       Windows 11\nFirewall:       Windows Defender Firewall\nPower Scheme:   Active via powercfg",
            profile.to_uppercase()
        )
    }

    #[cfg(unix)]
    {
        let ptrace = execute("sysctl", &["-n", "kernel.yama.ptrace_scope"]).unwrap_or_else(|_| "N/A".into());
        let inotify = execute("sysctl", &["-n", "fs.inotify.max_user_watches"]).unwrap_or_else(|_| "N/A".into());
        let map_count = execute("sysctl", &["-n", "vm.max_map_count"]).unwrap_or_else(|_| "N/A".into());
        let ufw_status = execute("ufw", &["status"]).unwrap_or_else(|_| "inactive".into());
        let cpu_epp = get_cpu_epp().unwrap_or_else(|| "N/A".into());
        let usb_lockdown = if is_usb_locked_down() { "ENGAGED (BadUSB Blocked)" } else { "Standard (Auto-authorized)" };
        let bt_status = if is_bluetooth_blocked() { "Blocked / Radio Off" } else { "Active / Unblocked" };
        let dns_status = get_dns_status();
        let dns_summary = if dns_status.is_encrypted {
            format!("🔒 Encrypted (DoT: {}, DNSSEC: {})", dns_status.dns_over_tls, dns_status.dnssec)
        } else if !dns_status.active_servers.is_empty() {
            format!("⚠️ Plaintext ({})", dns_status.active_servers.join(", "))
        } else {
            "System Default (DHCP)".to_string()
        };

        let cam_status = if is_camera_blocked() { "BLOCKED (Privacy Lock)" } else { "Enabled / Active" };
        let mic_status = if is_microphone_muted() { "MUTED (Hardware Lock)" } else { "Active / Unmuted" };

        format!(
            "Active Profile: {}\nCPU EPP Mode:   {}\nUSB Security:   {}\nBluetooth:      {}\nCamera Privacy: {}\nMic Privacy:    {}\nDNS Privacy:    {}\nptrace_scope:   {}\ninotify watches:{}\nvm.max_map_count: {}\nUFW Status:\n{}",
            profile.to_uppercase(),
            cpu_epp,
            usb_lockdown,
            bt_status,
            cam_status,
            mic_status,
            dns_summary,
            ptrace,
            inotify,
            map_count,
            ufw_status
        )
    }
}

pub fn block_port(port: u16, proto: &str) -> Result<(), String> {
    if !is_privileged() {
        return Err("Root/Administrator privileges required to block ports".to_string());
    }

    #[cfg(windows)]
    {
        let rule_name = format!("PostureFlow-Block-{}", port);
        let _ = Command::new("netsh")
            .args([
                "advfirewall", "firewall", "add", "rule",
                &format!("name={}", rule_name),
                "dir=in", "action=block",
                &format!("protocol={}", proto.to_uppercase()),
                &format!("localport={}", port),
                "enable=yes",
                "description=Blocked via PostureFlow Port Inspector",
            ])
            .output();
        Ok(())
    }

    #[cfg(unix)]
    {
        let rule = format!("{}/{}", port, proto);
        execute("ufw", &["deny", &rule, "comment", "Blocked via PostureFlow Port Inspector"])?;
        let _ = execute("ufw", &["reload"]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_status_report_default() {
        let rep = DnsStatusReport::default();
        assert!(!rep.is_encrypted);
        assert_eq!(rep.dns_over_tls, "");
        assert_eq!(rep.dnssec, "");
        assert!(rep.active_servers.is_empty());
        assert!(!rep.managed_by_postureflow);
    }

    #[test]
    fn test_dns_status_report_serialization() {
        let rep = DnsStatusReport {
            is_encrypted: true,
            dns_over_tls: "yes".to_string(),
            dnssec: "yes".to_string(),
            active_servers: vec!["9.9.9.9#dns.quad9.net".to_string()],
            managed_by_postureflow: true,
        };
        let json = serde_json::to_string(&rep).unwrap();
        let deserialized: DnsStatusReport = serde_json::from_str(&json).unwrap();
        assert_eq!(rep, deserialized);
    }
}


