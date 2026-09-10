use super::schema::ProfileConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub warnings: Vec<String>,
    pub sanitized_config: ProfileConfig,
}

pub fn validate_and_sanitize(mut config: ProfileConfig) -> Result<ValidationReport, String> {
    let mut warnings = Vec::new();

    // 1. Validate ID
    let id = config.profile.id.trim().to_lowercase();
    if id.is_empty() {
        return Err("Profile ID cannot be empty".to_string());
    }
    if id.len() > 32 {
        return Err("Profile ID cannot exceed 32 characters".to_string());
    }
    if !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_') {
        return Err(format!(
            "Invalid profile ID '{}'. Only lowercase letters, digits, '-', and '_' are allowed.",
            id
        ));
    }
    config.profile.id = id;

    // 2. Validate Name
    config.profile.name = config.profile.name.trim().to_string();
    if config.profile.name.is_empty() {
        return Err("Profile name cannot be empty".to_string());
    }
    if config.profile.name.len() > 64 {
        return Err("Profile name cannot exceed 64 characters".to_string());
    }

    // 3. Safety Invariant: Loopback must never be disabled
    if !config.firewall.allow_loopback {
        warnings.push("Firewall 'allow_loopback' was false; enforced to true to protect system IPC.".to_string());
        config.firewall.allow_loopback = true;
    }

    // 4. Validate Firewall Policies
    let inc = config.firewall.default_incoming.trim().to_lowercase();
    if inc != "deny" && inc != "allow" && inc != "reject" {
        return Err(format!(
            "Invalid default_incoming '{}'. Expected 'deny', 'allow', or 'reject'.",
            inc
        ));
    }
    config.firewall.default_incoming = inc;

    let out = config.firewall.default_outgoing.trim().to_lowercase();
    if out != "allow" && out != "deny" {
        return Err(format!(
            "Invalid default_outgoing '{}'. Expected 'allow' or 'deny'.",
            out
        ));
    }
    config.firewall.default_outgoing = out;

    // 5. Validate Port Rules
    for port_rule in &config.firewall.allow_ports {
        let p = port_rule.port.trim();
        if p.is_empty() {
            return Err("Port rule cannot be empty".to_string());
        }

        let parts: Vec<&str> = p.split('/').collect();
        let port_part = parts[0];
        if parts.len() > 2 {
            return Err(format!("Invalid port format '{}'. Expected '<port>/<protocol>'.", p));
        }
        if parts.len() == 2 {
            let proto = parts[1].to_lowercase();
            if proto != "tcp" && proto != "udp" {
                return Err(format!("Invalid protocol '{}' in port '{}'. Expected 'tcp' or 'udp'.", proto, p));
            }
        }

        // Check single port or range
        if port_part.contains(':') {
            let range: Vec<&str> = port_part.split(':').collect();
            if range.len() != 2 {
                return Err(format!("Invalid port range '{}'. Expected '<start>:<end>'.", port_part));
            }
            let start: u16 = range[0].parse().map_err(|_| format!("Invalid start port '{}'", range[0]))?;
            let end: u16 = range[1].parse().map_err(|_| format!("Invalid end port '{}'", range[1]))?;
            if start == 0 || end == 0 || start > end {
                return Err(format!("Invalid port range '{}:{}'. Start must be <= end.", start, end));
            }
        } else {
            let port_num: u16 = port_part.parse().map_err(|_| format!("Invalid port number '{}'", port_part))?;
            if port_num == 0 {
                return Err("Port number cannot be 0".to_string());
            }
        }
    }

    // 6. Whitelist & Validate Sysctl Keys
    let allowed_prefixes = ["kernel.", "fs.", "vm.", "net.ipv4.", "net.ipv6."];
    for (key, val) in &config.kernel {
        let is_allowed = allowed_prefixes.iter().any(|pref| key.starts_with(pref));
        if !is_allowed {
            return Err(format!(
                "Sysctl key '{}' is not permitted. Only kernel.*, fs.*, vm.*, net.ipv4.*, and net.ipv6.* are allowed.",
                key
            ));
        }

        // Disallow dangerous keys
        if key == "kernel.core_pattern" && val.starts_with('|') {
            return Err("Piping kernel.core_pattern to arbitrary programs is disallowed for security.".to_string());
        }

        if key == "kernel.unprivileged_bpf_disabled" && val == "1" {
            warnings.push("kernel.unprivileged_bpf_disabled=1 irreversibly locks eBPF until reboot. Adjusted to 2 (admin-managed).".to_string());
        }
    }

    // If eBPF was set to 1, sanitize to 2
    if let Some(bpf) = config.kernel.get_mut("kernel.unprivileged_bpf_disabled") {
        if bpf == "1" {
            *bpf = "2".to_string();
        }
    }

    // 7. Validate Framework battery charge limit if present
    if let Some(limit) = config.power.battery_charge_limit {
        if !(20..=100).contains(&limit) {
            return Err(format!(
                "Invalid battery_charge_limit '{}%'. Expected a value between 20 and 100.",
                limit
            ));
        }
    }

    // 8. Validate power profile if present
    if let Some(ref prof) = config.power.power_profile {
        let prof_lower = prof.to_lowercase();
        if prof_lower != "battery" && prof_lower != "balanced" && prof_lower != "performance" {
            return Err(format!(
                "Invalid power_profile '{}'. Expected 'battery', 'balanced', or 'performance'.",
                prof
            ));
        }
    }

    // 9. Validate CPU EPP if present
    if let Some(ref epp) = config.power.cpu_epp {
        let epp_lower = epp.trim().to_lowercase();
        if !["default", "performance", "balance_performance", "balance_power", "power"].contains(&epp_lower.as_str()) {
            return Err(format!(
                "Invalid cpu_epp '{}'. Expected 'default', 'performance', 'balance_performance', 'balance_power', or 'power'.",
                epp
            ));
        }
        config.power.cpu_epp = Some(epp_lower);
    }

    Ok(ValidationReport {
        is_valid: true,
        warnings,
        sanitized_config: config,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::*;
    use std::collections::HashMap;

    fn sample_config() -> ProfileConfig {
        ProfileConfig {
            profile: ProfileMetadata {
                id: "ai-dev".to_string(),
                name: "AI Development".to_string(),
                description: "Test description".to_string(),
                icon: "utilities-terminal-symbolic".to_string(),
                version: "1.0.0".to_string(),
                author: "Test".to_string(),
                is_builtin: false,
            },
            kernel: HashMap::new(),
            limits: SecurityLimitsConfig::default(),
            firewall: FirewallConfig::default(),
            power: FrameworkPowerConfig::default(),
            peripherals: PeripheralsConfig::default(),
            desktop: DesktopConfig::default(),
        }
    }

    #[test]
    fn test_valid_profile() {
        let mut cfg = sample_config();
        cfg.firewall.allow_ports.push(PortRule {
            port: "8080/tcp".to_string(),
            comment: "Web".to_string(),
        });
        cfg.kernel.insert("vm.max_map_count".to_string(), "2147483642".to_string());

        let res = validate_and_sanitize(cfg);
        assert!(res.is_ok());
        let report = res.unwrap();
        assert!(report.is_valid);
        assert_eq!(report.sanitized_config.profile.id, "ai-dev");
    }

    #[test]
    fn test_loopback_safety_enforced() {
        let mut cfg = sample_config();
        cfg.firewall.allow_loopback = false;

        let res = validate_and_sanitize(cfg).unwrap();
        assert!(res.sanitized_config.firewall.allow_loopback, "Loopback must be enforced to true");
        assert!(!res.warnings.is_empty());
    }

    #[test]
    fn test_bpf_lock_prevented() {
        let mut cfg = sample_config();
        cfg.kernel.insert("kernel.unprivileged_bpf_disabled".to_string(), "1".to_string());

        let res = validate_and_sanitize(cfg).unwrap();
        assert_eq!(
            res.sanitized_config.kernel.get("kernel.unprivileged_bpf_disabled").unwrap(),
            "2",
            "eBPF lock value 1 must be sanitized to 2"
        );
    }

    #[test]
    fn test_disallow_core_pattern_pipe() {
        let mut cfg = sample_config();
        cfg.kernel.insert("kernel.core_pattern".to_string(), "|/tmp/malicious.sh".to_string());

        let res = validate_and_sanitize(cfg);
        assert!(res.is_err());
    }

    #[test]
    fn test_disallow_unwhitelisted_sysctl() {
        let mut cfg = sample_config();
        cfg.kernel.insert("user.max_user_namespaces".to_string(), "0".to_string());

        let res = validate_and_sanitize(cfg);
        assert!(res.is_err());
    }

    #[test]
    fn test_invalid_port_format() {
        let mut cfg = sample_config();
        cfg.firewall.allow_ports.push(PortRule {
            port: "invalid_port".to_string(),
            comment: "".to_string(),
        });
        assert!(validate_and_sanitize(cfg).is_err());
    }

    #[test]
    fn test_cpu_epp_validation() {
        let mut cfg = sample_config();
        cfg.power.cpu_epp = Some("performance".to_string());
        let res = validate_and_sanitize(cfg.clone());
        assert!(res.is_ok());

        cfg.power.cpu_epp = Some("invalid_mode".to_string());
        let res_err = validate_and_sanitize(cfg);
        assert!(res_err.is_err());
    }
}
