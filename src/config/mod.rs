pub mod schema;
pub mod validator;

pub use schema::{
    DesktopConfig, FirewallConfig, FrameworkPowerConfig, PeripheralsConfig, PortRule, ProfileConfig,
    ProfileMetadata, SecurityLimitsConfig,
};
pub use validator::{validate_and_sanitize, ValidationReport};

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const SYSTEM_PROFILES_DIR: &str = "/etc/postureflow/profiles.d";

pub fn user_profiles_dir() -> PathBuf {
    if let Ok(path) = std::env::var("POSTUREFLOW_CUSTOM_DIR") {
        return PathBuf::from(path);
    }
    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(config_home).join("postureflow/profiles.d")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/postureflow/profiles.d")
    } else {
        PathBuf::from("/tmp/postureflow/profiles.d")
    }
}

pub fn builtin_profiles() -> Vec<ProfileConfig> {
    vec![
        builtin_home(),
        builtin_work(),
        builtin_dev(),
        builtin_travel(),
    ]
}

fn builtin_home() -> ProfileConfig {
    let mut kernel = HashMap::new();
    kernel.insert("kernel.yama.ptrace_scope".to_string(), "1".to_string());
    kernel.insert("kernel.dmesg_restrict".to_string(), "0".to_string());
    kernel.insert("kernel.kptr_restrict".to_string(), "1".to_string());
    kernel.insert("kernel.unprivileged_bpf_disabled".to_string(), "2".to_string());
    kernel.insert("fs.suid_dumpable".to_string(), "2".to_string());
    kernel.insert("vm.max_map_count".to_string(), "1048576".to_string());
    kernel.insert("fs.inotify.max_user_watches".to_string(), "524288".to_string());
    kernel.insert("fs.inotify.max_user_instances".to_string(), "512".to_string());
    kernel.insert("net.ipv4.conf.all.rp_filter".to_string(), "1".to_string());
    kernel.insert("net.ipv4.conf.default.rp_filter".to_string(), "1".to_string());
    kernel.insert("net.ipv4.tcp_syncookies".to_string(), "1".to_string());
    kernel.insert("net.ipv4.tcp_rfc1337".to_string(), "1".to_string());

    let ports = vec![
        PortRule { port: "27031:27036/udp".to_string(), comment: "Steam Remote Play".to_string() },
        PortRule { port: "27036:27037/tcp".to_string(), comment: "Steam Remote Play".to_string() },
        PortRule { port: "27040/tcp".to_string(), comment: "Steam LAN Game Download".to_string() },
        PortRule { port: "1714:1764/tcp".to_string(), comment: "GSConnect Phone Sync".to_string() },
        PortRule { port: "1714:1764/udp".to_string(), comment: "GSConnect Phone Sync".to_string() },
        PortRule { port: "53317/tcp".to_string(), comment: "LocalSend File Share".to_string() },
        PortRule { port: "53317/udp".to_string(), comment: "LocalSend File Share".to_string() },
        PortRule { port: "5353/udp".to_string(), comment: "mDNS Chromecast".to_string() },
    ];

    ProfileConfig {
        profile: ProfileMetadata {
            id: "home".to_string(),
            name: "Home / Streaming / Gaming".to_string(),
            description: "LAN discovery, Steam Remote Play, GSConnect, and relaxed gaming map counts.".to_string(),
            icon: "user-home-symbolic".to_string(),
            version: "1.0.0".to_string(),
            author: "Pop!_OS".to_string(),
            is_builtin: true,
        },
        kernel,
        limits: SecurityLimitsConfig::default(),
        firewall: FirewallConfig {
            default_incoming: "deny".to_string(),
            default_outgoing: "allow".to_string(),
            allow_loopback: true,
            allow_interfaces: Vec::new(),
            allow_ports: ports,
        },
        power: FrameworkPowerConfig {
            battery_charge_limit: Some(85),
            power_profile: Some("balanced".to_string()),
            cpu_epp: Some("balance_performance".to_string()),
        },
        peripherals: PeripheralsConfig {
            block_new_usb: Some(false),
            bluetooth: Some(true),
        },
        desktop: DesktopConfig { idle_delay_seconds: Some(1800) },
    }
}

fn builtin_work() -> ProfileConfig {
    let mut kernel = HashMap::new();
    kernel.insert("kernel.yama.ptrace_scope".to_string(), "1".to_string());
    kernel.insert("kernel.dmesg_restrict".to_string(), "1".to_string());
    kernel.insert("kernel.kptr_restrict".to_string(), "1".to_string());
    kernel.insert("kernel.unprivileged_bpf_disabled".to_string(), "2".to_string());
    kernel.insert("fs.suid_dumpable".to_string(), "0".to_string());
    kernel.insert("fs.inotify.max_user_watches".to_string(), "524288".to_string());
    kernel.insert("fs.inotify.max_user_instances".to_string(), "512".to_string());
    kernel.insert("net.ipv4.conf.all.rp_filter".to_string(), "1".to_string());
    kernel.insert("net.ipv4.conf.default.rp_filter".to_string(), "1".to_string());
    kernel.insert("net.ipv4.conf.all.accept_redirects".to_string(), "0".to_string());
    kernel.insert("net.ipv4.conf.default.accept_redirects".to_string(), "0".to_string());
    kernel.insert("net.ipv6.conf.all.accept_redirects".to_string(), "0".to_string());
    kernel.insert("net.ipv6.conf.default.accept_redirects".to_string(), "0".to_string());
    kernel.insert("net.ipv4.tcp_syncookies".to_string(), "1".to_string());
    kernel.insert("net.ipv4.tcp_rfc1337".to_string(), "1".to_string());

    let ports = vec![
        PortRule { port: "631/tcp".to_string(), comment: "Office Printing (CUPS)".to_string() },
        PortRule { port: "5353/udp".to_string(), comment: "Office mDNS Discovery".to_string() },
    ];

    ProfileConfig {
        profile: ProfileMetadata {
            id: "work".to_string(),
            name: "Work / Office / Corporate VPN".to_string(),
            description: "Strict network redirects, corporate VPN passthrough, CUPS printing, and 5-minute screen lock.".to_string(),
            icon: "applications-office-symbolic".to_string(),
            version: "1.0.0".to_string(),
            author: "Pop!_OS".to_string(),
            is_builtin: true,
        },
        kernel,
        limits: SecurityLimitsConfig::default(),
        firewall: FirewallConfig {
            default_incoming: "deny".to_string(),
            default_outgoing: "allow".to_string(),
            allow_loopback: true,
            allow_interfaces: vec!["tun+".to_string(), "tap+".to_string(), "wg+".to_string(), "tailscale+".to_string()],
            allow_ports: ports,
        },
        power: FrameworkPowerConfig {
            battery_charge_limit: Some(80),
            power_profile: Some("balanced".to_string()),
            cpu_epp: Some("balance_performance".to_string()),
        },
        peripherals: PeripheralsConfig {
            block_new_usb: Some(false),
            bluetooth: Some(true),
        },
        desktop: DesktopConfig { idle_delay_seconds: Some(300) },
    }
}

fn builtin_dev() -> ProfileConfig {
    let mut kernel = HashMap::new();
    kernel.insert("kernel.yama.ptrace_scope".to_string(), "1".to_string());
    kernel.insert("kernel.dmesg_restrict".to_string(), "0".to_string());
    kernel.insert("kernel.kptr_restrict".to_string(), "1".to_string());
    kernel.insert("kernel.unprivileged_bpf_disabled".to_string(), "2".to_string());
    kernel.insert("fs.suid_dumpable".to_string(), "2".to_string());
    kernel.insert("fs.inotify.max_user_watches".to_string(), "524288".to_string());
    kernel.insert("fs.inotify.max_user_instances".to_string(), "512".to_string());
    kernel.insert("net.ipv4.conf.all.rp_filter".to_string(), "1".to_string());
    kernel.insert("net.ipv4.conf.default.rp_filter".to_string(), "1".to_string());
    kernel.insert("net.ipv4.tcp_syncookies".to_string(), "1".to_string());
    kernel.insert("net.ipv4.tcp_rfc1337".to_string(), "1".to_string());

    let ports = vec![
        PortRule { port: "3000/tcp".to_string(), comment: "Node/React/Frontend".to_string() },
        PortRule { port: "5000/tcp".to_string(), comment: "Flask/ASP.NET".to_string() },
        PortRule { port: "5173/tcp".to_string(), comment: "Vite Dev Server".to_string() },
        PortRule { port: "8000/tcp".to_string(), comment: "Django/Python/FastAPI".to_string() },
        PortRule { port: "8080/tcp".to_string(), comment: "Spring/Tomcat/Proxy".to_string() },
        PortRule { port: "8888/tcp".to_string(), comment: "JupyterLab".to_string() },
        PortRule { port: "9000/tcp".to_string(), comment: "PHP-FPM/SonarQube".to_string() },
    ];

    ProfileConfig {
        profile: ProfileMetadata {
            id: "dev".to_string(),
            name: "Developer / Coding Mode".to_string(),
            description: "Inotify watches boosted, core dumps enabled, dev container interfaces and local dev ports unblocked.".to_string(),
            icon: "utilities-terminal-symbolic".to_string(),
            version: "1.0.0".to_string(),
            author: "Pop!_OS".to_string(),
            is_builtin: true,
        },
        kernel,
        limits: SecurityLimitsConfig {
            core_dump: Some("unlimited".to_string()),
            nofile_soft: Some(65536),
            nofile_hard: Some(1048576),
        },
        firewall: FirewallConfig {
            default_incoming: "deny".to_string(),
            default_outgoing: "allow".to_string(),
            allow_loopback: true,
            allow_interfaces: vec!["docker0".to_string(), "podman0".to_string(), "virbr0".to_string(), "cni0".to_string()],
            allow_ports: ports,
        },
        power: FrameworkPowerConfig {
            battery_charge_limit: Some(85),
            power_profile: Some("performance".to_string()),
            cpu_epp: Some("performance".to_string()),
        },
        peripherals: PeripheralsConfig {
            block_new_usb: Some(false),
            bluetooth: Some(true),
        },
        desktop: DesktopConfig { idle_delay_seconds: Some(1800) },
    }
}

fn builtin_travel() -> ProfileConfig {
    let mut kernel = HashMap::new();
    kernel.insert("kernel.yama.ptrace_scope".to_string(), "2".to_string());
    kernel.insert("kernel.dmesg_restrict".to_string(), "1".to_string());
    kernel.insert("kernel.kptr_restrict".to_string(), "2".to_string());
    kernel.insert("kernel.unprivileged_bpf_disabled".to_string(), "2".to_string());
    kernel.insert("fs.suid_dumpable".to_string(), "0".to_string());
    kernel.insert("fs.inotify.max_user_watches".to_string(), "131072".to_string());
    kernel.insert("fs.inotify.max_user_instances".to_string(), "256".to_string());
    kernel.insert("net.ipv4.conf.all.rp_filter".to_string(), "1".to_string());
    kernel.insert("net.ipv4.conf.default.rp_filter".to_string(), "1".to_string());
    kernel.insert("net.ipv4.conf.all.accept_redirects".to_string(), "0".to_string());
    kernel.insert("net.ipv4.conf.default.accept_redirects".to_string(), "0".to_string());
    kernel.insert("net.ipv6.conf.all.accept_redirects".to_string(), "0".to_string());
    kernel.insert("net.ipv6.conf.default.accept_redirects".to_string(), "0".to_string());
    kernel.insert("net.ipv4.tcp_syncookies".to_string(), "1".to_string());
    kernel.insert("net.ipv4.tcp_rfc1337".to_string(), "1".to_string());
    kernel.insert("net.ipv4.icmp_echo_ignore_broadcasts".to_string(), "1".to_string());
    kernel.insert("net.ipv4.icmp_ignore_bogus_error_responses".to_string(), "1".to_string());

    ProfileConfig {
        profile: ProfileMetadata {
            id: "travel".to_string(),
            name: "Travel / Public Wi-Fi Lockdown".to_string(),
            description: "Zero open ports, strict ICMP ignoring, aggressive ptrace prevention, and 2-minute screen lock.".to_string(),
            icon: "security-high-symbolic".to_string(),
            version: "1.0.0".to_string(),
            author: "Pop!_OS".to_string(),
            is_builtin: true,
        },
        kernel,
        limits: SecurityLimitsConfig {
            core_dump: Some("0".to_string()),
            nofile_soft: None,
            nofile_hard: None,
        },
        firewall: FirewallConfig {
            default_incoming: "deny".to_string(),
            default_outgoing: "allow".to_string(),
            allow_loopback: true,
            allow_interfaces: Vec::new(),
            allow_ports: Vec::new(),
        },
        power: FrameworkPowerConfig {
            battery_charge_limit: Some(80),
            power_profile: Some("battery".to_string()),
            cpu_epp: Some("power".to_string()),
        },
        peripherals: PeripheralsConfig {
            block_new_usb: Some(true),
            bluetooth: Some(true),
        },
        desktop: DesktopConfig { idle_delay_seconds: Some(120) },
    }
}

pub fn load_all_profiles() -> Vec<ProfileConfig> {
    let mut profiles = builtin_profiles();
    let mut ids: std::collections::HashSet<String> = profiles.iter().map(|p| p.profile.id.clone()).collect();

    // 1. Scan System directory (/etc/postureflow/profiles.d)
    scan_directory(Path::new(SYSTEM_PROFILES_DIR), &mut profiles, &mut ids);

    // 2. Scan User directory (~/.config/postureflow/profiles.d)
    let user_dir = user_profiles_dir();
    scan_directory(&user_dir, &mut profiles, &mut ids);

    profiles
}

fn scan_directory(
    dir: &Path,
    profiles: &mut Vec<ProfileConfig>,
    seen_ids: &mut std::collections::HashSet<String>,
) {
    if !dir.exists() || !dir.is_dir() {
        return;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(config) = ProfileConfig::from_toml(&content) {
                        if let Ok(report) = validate_and_sanitize(config) {
                            let sanitized = report.sanitized_config;
                            if seen_ids.insert(sanitized.profile.id.clone()) {
                                profiles.push(sanitized);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn find_profile(id: &str) -> Option<ProfileConfig> {
    let mut normalized = id.trim().to_lowercase();
    if normalized == "secure" {
        normalized = "travel".to_string();
    }
    load_all_profiles()
        .into_iter()
        .find(|p| p.profile.id.to_lowercase() == normalized)
}

pub fn save_custom_profile(config: ProfileConfig, is_system: bool) -> Result<PathBuf, String> {
    let report = validate_and_sanitize(config)?;
    let sanitized = report.sanitized_config;

    if sanitized.profile.is_builtin {
        return Err("Built-in profiles cannot be overwritten.".to_string());
    }

    let target_dir = if is_system {
        PathBuf::from(SYSTEM_PROFILES_DIR)
    } else {
        user_profiles_dir()
    };

    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create directory {}: {}", target_dir.display(), e))?;

    let file_path = target_dir.join(format!("{}.toml", sanitized.profile.id));
    let toml_str = sanitized
        .to_toml()
        .map_err(|e| format!("Failed to serialize TOML: {}", e))?;

    fs::write(&file_path, toml_str)
        .map_err(|e| format!("Failed to write {}: {}", file_path.display(), e))?;

    Ok(file_path)
}

pub fn delete_custom_profile(id: &str) -> Result<(), String> {
    let normalized = id.trim().to_lowercase();
    if ["home", "work", "dev", "travel", "secure"].contains(&normalized.as_str()) {
        return Err("Cannot delete built-in system profile.".to_string());
    }

    let mut deleted = false;

    // Check system dir
    let sys_path = Path::new(SYSTEM_PROFILES_DIR).join(format!("{}.toml", normalized));
    if sys_path.exists() {
        fs::remove_file(&sys_path)
            .map_err(|e| format!("Failed to delete {}: {}", sys_path.display(), e))?;
        deleted = true;
    }

    // Check user dir
    let user_path = user_profiles_dir().join(format!("{}.toml", normalized));
    if user_path.exists() {
        fs::remove_file(&user_path)
            .map_err(|e| format!("Failed to delete {}: {}", user_path.display(), e))?;
        deleted = true;
    }

    if deleted {
        Ok(())
    } else {
        Err(format!("Custom profile '{}' not found.", normalized))
    }
}
