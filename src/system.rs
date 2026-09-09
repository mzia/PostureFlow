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

pub fn save_active_profile(profile: Profile) -> Result<(), String> {
    let path = state_file_path();
    fs::write(&path, profile.as_str())
        .map_err(|e| format!("Failed to write state file {}: {}", path, e))
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
    match profile {
        Profile::Home => apply_home(),
        Profile::Work => apply_work(),
        Profile::Dev => apply_dev(),
        Profile::Secure => apply_secure(),
    }?;

    save_active_profile(profile)?;
    Ok(())
}

fn apply_home() -> Result<(), String> {
    if !is_privileged() {
        return Ok(());
    }
    let sysctl_content = "\
kernel.yama.ptrace_scope = 1
kernel.dmesg_restrict = 0
kernel.kptr_restrict = 1
kernel.unprivileged_bpf_disabled = 2
fs.suid_dumpable = 2
vm.max_map_count = 1048576
fs.inotify.max_user_watches = 524288
fs.inotify.max_user_instances = 512
net.ipv4.conf.all.rp_filter = 1
net.ipv4.conf.default.rp_filter = 1
net.ipv4.tcp_syncookies = 1
net.ipv4.tcp_rfc1337 = 1
";
    fs::write(SYSCTL_CONF, sysctl_content).map_err(|e| format!("Writing sysctl failed: {}", e))?;
    let _ = execute("sysctl", &["-p", SYSCTL_CONF]);

    let _ = execute("ufw", &["--force", "default", "deny", "incoming"]);
    let _ = execute("ufw", &["--force", "default", "allow", "outgoing"]);
    let _ = execute("ufw", &["allow", "in", "on", "lo"]);
    let _ = execute("ufw", &["allow", "out", "on", "lo"]);

    clean_ufw_custom_rules();

    let _ = execute("ufw", &["allow", "27031:27036/udp", "comment", "Steam Remote Play"]);
    let _ = execute("ufw", &["allow", "27036:27037/tcp", "comment", "Steam Remote Play"]);
    let _ = execute("ufw", &["allow", "27040/tcp", "comment", "Steam LAN Game Download"]);
    let _ = execute("ufw", &["allow", "1714:1764/tcp", "comment", "GSConnect Phone Sync"]);
    let _ = execute("ufw", &["allow", "1714:1764/udp", "comment", "GSConnect Phone Sync"]);
    let _ = execute("ufw", &["allow", "53317/tcp", "comment", "LocalSend File Share"]);
    let _ = execute("ufw", &["allow", "53317/udp", "comment", "LocalSend File Share"]);
    let _ = execute("ufw", &["allow", "5353/udp", "comment", "mDNS Chromecast"]);

    ensure_ssh_safety();
    let _ = execute("ufw", &["--force", "enable"]);

    set_desktop_idle_delay(1800);
    Ok(())
}

fn apply_work() -> Result<(), String> {
    if !is_privileged() {
        return Ok(());
    }
    let sysctl_content = "\
kernel.yama.ptrace_scope = 1
kernel.dmesg_restrict = 1
kernel.kptr_restrict = 1
kernel.unprivileged_bpf_disabled = 2
fs.suid_dumpable = 0
fs.inotify.max_user_watches = 524288
fs.inotify.max_user_instances = 512
net.ipv4.conf.all.rp_filter = 1
net.ipv4.conf.default.rp_filter = 1
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.default.accept_redirects = 0
net.ipv6.conf.all.accept_redirects = 0
net.ipv6.conf.default.accept_redirects = 0
net.ipv4.tcp_syncookies = 1
net.ipv4.tcp_rfc1337 = 1
";
    fs::write(SYSCTL_CONF, sysctl_content).map_err(|e| format!("Writing sysctl failed: {}", e))?;
    let _ = execute("sysctl", &["-p", SYSCTL_CONF]);

    let _ = execute("ufw", &["--force", "default", "deny", "incoming"]);
    let _ = execute("ufw", &["--force", "default", "allow", "outgoing"]);
    let _ = execute("ufw", &["allow", "in", "on", "lo"]);
    let _ = execute("ufw", &["allow", "out", "on", "lo"]);

    clean_ufw_custom_rules();

    for iface in ["tun+", "tap+", "wg+", "tailscale+"] {
        let _ = execute("ufw", &["allow", "in", "on", iface, "comment", "Corporate VPN"]);
        let _ = execute("ufw", &["allow", "out", "on", iface, "comment", "Corporate VPN"]);
    }
    let _ = execute("ufw", &["allow", "631/tcp", "comment", "Office Printing (CUPS)"]);
    let _ = execute("ufw", &["allow", "5353/udp", "comment", "Office mDNS Discovery"]);

    ensure_ssh_safety();
    let _ = execute("ufw", &["--force", "enable"]);

    set_desktop_idle_delay(300);
    Ok(())
}

fn apply_dev() -> Result<(), String> {
    if !is_privileged() {
        return Ok(());
    }
    let sysctl_content = "\
kernel.yama.ptrace_scope = 1
kernel.dmesg_restrict = 0
kernel.kptr_restrict = 1
kernel.unprivileged_bpf_disabled = 2
fs.suid_dumpable = 2
fs.inotify.max_user_watches = 524288
fs.inotify.max_user_instances = 512
net.ipv4.conf.all.rp_filter = 1
net.ipv4.conf.default.rp_filter = 1
net.ipv4.tcp_syncookies = 1
net.ipv4.tcp_rfc1337 = 1
";
    fs::write(SYSCTL_CONF, sysctl_content).map_err(|e| format!("Writing sysctl failed: {}", e))?;
    let _ = execute("sysctl", &["-p", SYSCTL_CONF]);

    let _ = fs::write(LIMITS_CONF, "* soft core unlimited\n");

    let _ = execute("ufw", &["--force", "default", "deny", "incoming"]);
    let _ = execute("ufw", &["--force", "default", "allow", "outgoing"]);
    let _ = execute("ufw", &["allow", "in", "on", "lo"]);
    let _ = execute("ufw", &["allow", "out", "on", "lo"]);

    clean_ufw_custom_rules();

    for iface in ["docker0", "podman0"] {
        if std::path::Path::new(&format!("/sys/class/net/{}", iface)).exists() {
            let _ = execute("ufw", &["allow", "in", "on", iface]);
            let _ = execute("ufw", &["allow", "out", "on", iface]);
        }
    }

    let ports = ["3000", "5000", "5173", "8000", "8080", "8888", "9000"];
    for p in ports {
        let _ = execute("ufw", &["allow", &format!("{}/tcp", p), "comment", "Dev local port"]);
    }

    ensure_ssh_safety();
    let _ = execute("ufw", &["--force", "enable"]);

    set_desktop_idle_delay(900);
    Ok(())
}

fn apply_secure() -> Result<(), String> {
    if !is_privileged() {
        return Ok(());
    }
    let sysctl_content = "\
kernel.yama.ptrace_scope = 2
kernel.dmesg_restrict = 1
kernel.kptr_restrict = 2
kernel.unprivileged_bpf_disabled = 2
fs.suid_dumpable = 0
net.ipv4.conf.all.rp_filter = 1
net.ipv4.conf.default.rp_filter = 1
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.default.accept_redirects = 0
net.ipv6.conf.all.accept_redirects = 0
net.ipv6.conf.default.accept_redirects = 0
net.ipv4.conf.all.send_redirects = 0
net.ipv4.conf.default.send_redirects = 0
net.ipv4.conf.all.accept_source_route = 0
net.ipv4.conf.default.accept_source_route = 0
net.ipv6.conf.all.accept_source_route = 0
net.ipv6.conf.default.accept_source_route = 0
net.ipv4.icmp_echo_ignore_broadcasts = 1
net.ipv4.tcp_syncookies = 1
net.ipv4.tcp_rfc1337 = 1
";
    fs::write(SYSCTL_CONF, sysctl_content).map_err(|e| format!("Writing sysctl failed: {}", e))?;
    let _ = execute("sysctl", &["-p", SYSCTL_CONF]);

    let _ = fs::write(LIMITS_CONF, "* hard core 0\n* soft core 0\n");

    let _ = execute("ufw", &["--force", "default", "deny", "incoming"]);
    let _ = execute("ufw", &["--force", "default", "allow", "outgoing"]);
    let _ = execute("ufw", &["allow", "in", "on", "lo"]);
    let _ = execute("ufw", &["allow", "out", "on", "lo"]);

    clean_ufw_custom_rules();
    ensure_ssh_safety();

    let _ = execute("ufw", &["logging", "low"]);
    let _ = execute("ufw", &["--force", "enable"]);

    set_desktop_idle_delay(300);
    Ok(())
}

pub fn reset_to_defaults() -> Result<(), String> {
    let path = state_file_path();
    let _ = fs::remove_file(&path);

    if !is_privileged() {
        return Ok(());
    }

    let _ = execute("ufw", &["--force", "disable"]);
    let _ = execute("ufw", &["--force", "reset"]);

    let _ = fs::remove_file(SYSCTL_CONF);
    let _ = fs::remove_file(LIMITS_CONF);

    let _ = execute("sysctl", &["--system"]);
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
