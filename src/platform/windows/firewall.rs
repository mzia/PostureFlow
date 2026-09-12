use std::process::Command;
use crate::config::ProfileConfig;

pub fn apply_windows_firewall(config: &ProfileConfig) -> Result<(), String> {
    // 1. Inbound/Outbound default policy
    let inbound_policy = match config.firewall.default_incoming.as_str() {
        "allow" => "allowinbound",
        _ => "blockinbound",
    };
    let outbound_policy = match config.firewall.default_outgoing.as_str() {
        "deny" | "block" => "blockoutbound",
        _ => "allowoutbound",
    };

    let _ = Command::new("netsh")
        .args([
            "advfirewall", "set", "allprofiles", "firewallpolicy",
            &format!("{},{}", inbound_policy, outbound_policy),
        ])
        .output();

    // 2. Clean previous PostureFlow rules
    clean_postureflow_firewall_rules();

    // 3. Loopback safety (always allow localhost)
    if config.firewall.allow_loopback {
        let _ = Command::new("netsh")
            .args([
                "advfirewall", "firewall", "add", "rule",
                "name=PostureFlow-Loopback-In",
                "dir=in", "action=allow", "remoteip=127.0.0.1", "enable=yes",
            ])
            .output();
        let _ = Command::new("netsh")
            .args([
                "advfirewall", "firewall", "add", "rule",
                "name=PostureFlow-Loopback-Out",
                "dir=out", "action=allow", "remoteip=127.0.0.1", "enable=yes",
            ])
            .output();
    }

    // 4. Anti-lockout: RDP (3389) and SSH (22)
    ensure_windows_anti_lockout();

    // 5. Allow custom ports from profile
    for rule in &config.firewall.allow_ports {
        let port = &rule.port;
        let rule_name = format!("PostureFlow-Port-{}", port);
        let proto = if port.contains("udp") { "UDP" } else { "TCP" };
        let port_clean = port.trim_end_matches("/tcp").trim_end_matches("/udp");

        let mut args = vec![
            "advfirewall".to_string(),
            "firewall".to_string(),
            "add".to_string(),
            "rule".to_string(),
            format!("name={}", rule_name),
            "dir=in".to_string(),
            "action=allow".to_string(),
            format!("protocol={}", proto),
            format!("localport={}", port_clean),
            "enable=yes".to_string(),
        ];
        if !rule.comment.is_empty() {
            args.push(format!("description={}", rule.comment));
        }
        let _ = Command::new("netsh").args(&args).output();
    }

    // 6. Travel mode: Block ICMP Echo Requests (Stealth Mode)
    if config.profile.id == "travel" {
        let _ = Command::new("netsh")
            .args([
                "advfirewall", "firewall", "add", "rule",
                "name=PostureFlow-Block-ICMP",
                "protocol=icmpv4:8,any", "dir=in", "action=block", "enable=yes",
            ])
            .output();
    }

    Ok(())
}

pub fn clean_postureflow_firewall_rules() {
    let ports = ["3000", "5000", "5173", "8000", "8080", "8888", "9000"];
    for p in ports {
        let _ = Command::new("netsh")
            .args(["advfirewall", "firewall", "delete", "rule", &format!("name=PostureFlow-Port-{}", p)])
            .output();
    }
    let _ = Command::new("netsh")
        .args(["advfirewall", "firewall", "delete", "rule", "name=PostureFlow-Block-ICMP"])
        .output();
    let _ = Command::new("netsh")
        .args(["advfirewall", "firewall", "delete", "rule", "name=PostureFlow-Loopback-In"])
        .output();
    let _ = Command::new("netsh")
        .args(["advfirewall", "firewall", "delete", "rule", "name=PostureFlow-Loopback-Out"])
        .output();
}

pub fn ensure_windows_anti_lockout() {
    let _ = Command::new("netsh")
        .args([
            "advfirewall", "firewall", "add", "rule",
            "name=PostureFlow-AntiLockout-RDP",
            "dir=in", "action=allow", "protocol=TCP", "localport=3389", "enable=yes",
        ])
        .output();
    let _ = Command::new("netsh")
        .args([
            "advfirewall", "firewall", "add", "rule",
            "name=PostureFlow-AntiLockout-SSH",
            "dir=in", "action=allow", "protocol=TCP", "localport=22", "enable=yes",
        ])
        .output();
}
