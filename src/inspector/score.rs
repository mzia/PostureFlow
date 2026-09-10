use serde::{Deserialize, Serialize};
use std::process::Command;
use super::ports::{self, ListeningPort};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostureScoreReport {
    pub total_score: u32,             // 0 - 100
    pub letter_grade: String,         // "A+", "A", "B", "C", "F"
    pub firewall_score: u32,          // 0 - 35
    pub attack_surface_score: u32,    // 0 - 25
    pub kernel_score: u32,            // 0 - 25
    pub limits_score: u32,            // 0 - 15
    pub ufw_active: bool,
    pub public_ports_count: usize,
    pub local_ports_count: usize,
    pub ptrace_scope: i32,
    pub bpf_disabled: i32,
    pub kptr_restrict: i32,
    pub syncookies: bool,
    pub rp_filter: i32,
    pub idle_timeout_seconds: u32,
    pub recommendations: Vec<String>,
    pub public_ports: Vec<ListeningPort>,
}

impl PostureScoreReport {
    pub fn compute() -> Self {
        let mut recommendations = Vec::new();

        // 1. Firewall Score (Max 35)
        let (ufw_active, default_deny) = check_ufw_status();
        let mut firewall_score = 0;
        if ufw_active {
            firewall_score += 25;
            if default_deny {
                firewall_score += 10;
            } else {
                recommendations.push("Default incoming firewall policy is permissive. Switch to deny/reject.".to_string());
            }
        } else {
            recommendations.push("UFW firewall is inactive. Enable firewall protection in Home, Work, or Travel profile.".to_string());
        }

        // 2. Attack Surface Score (Max 25)
        let ports = ports::scan_listening_ports();
        let public_ports: Vec<ListeningPort> = ports.iter().filter(|p| p.is_public).cloned().collect();
        let local_ports_count = ports.iter().filter(|p| p.is_local_only).count();
        let public_ports_count = public_ports.len();

        let attack_surface_score = match public_ports_count {
            0 => 25,
            1 => 18,
            2 => 12,
            3..=5 => 6,
            _ => 0,
        };

        if public_ports_count > 0 {
            recommendations.push(format!(
                "{} port(s) are actively exposed to LAN/WAN (0.0.0.0). Review in Port Inspector.",
                public_ports_count
            ));
        }

        // 3. Kernel Score (Max 25)
        let ptrace_scope = read_sysctl_i32("kernel.yama.ptrace_scope").unwrap_or(1);
        let bpf_disabled = read_sysctl_i32("kernel.unprivileged_bpf_disabled").unwrap_or(2);
        let kptr_restrict = read_sysctl_i32("kernel.kptr_restrict").unwrap_or(1);
        let syncookies = read_sysctl_i32("net.ipv4.tcp_syncookies").unwrap_or(1) == 1;
        let rp_filter = read_sysctl_i32("net.ipv4.conf.all.rp_filter").unwrap_or(1);

        let mut kernel_score = 0;
        match ptrace_scope {
            2 => kernel_score += 7,
            1 => kernel_score += 5,
            _ => recommendations.push("ptrace_scope is 0 (unrestricted memory reading). Set to 1 or 2.".to_string()),
        }

        if bpf_disabled >= 1 {
            kernel_score += 6;
        } else {
            recommendations.push("Unprivileged eBPF is enabled. Set kernel.unprivileged_bpf_disabled to 2.".to_string());
        }

        if kptr_restrict >= 1 {
            kernel_score += 4;
        }

        if syncookies {
            kernel_score += 4;
        }

        if rp_filter >= 1 {
            kernel_score += 4;
        }

        // 4. Desktop & System Limits (Max 15)
        let idle_timeout_seconds = read_screen_idle_delay();
        let suid_dumpable = read_sysctl_i32("fs.suid_dumpable").unwrap_or(2);

        let mut limits_score = 0;
        if idle_timeout_seconds <= 300 && idle_timeout_seconds > 0 {
            limits_score += 8;
        } else if idle_timeout_seconds <= 900 && idle_timeout_seconds > 0 {
            limits_score += 6;
        } else if idle_timeout_seconds <= 1800 && idle_timeout_seconds > 0 {
            limits_score += 4;
        } else {
            recommendations.push("Screen lock delay is longer than 30 minutes or disabled.".to_string());
        }

        match suid_dumpable {
            0 => limits_score += 7,
            2 => limits_score += 5, // Debugger allowed
            _ => limits_score += 2,
        }

        let total_score = firewall_score + attack_surface_score + kernel_score + limits_score;

        let letter_grade = match total_score {
            90..=100 => "A+".to_string(),
            80..=89 => "A".to_string(),
            70..=79 => "B".to_string(),
            55..=69 => "C".to_string(),
            _ => "F".to_string(),
        };

        Self {
            total_score,
            letter_grade,
            firewall_score,
            attack_surface_score,
            kernel_score,
            limits_score,
            ufw_active,
            public_ports_count,
            local_ports_count,
            ptrace_scope,
            bpf_disabled,
            kptr_restrict,
            syncookies,
            rp_filter,
            idle_timeout_seconds,
            recommendations,
            public_ports,
        }
    }
}

fn check_ufw_status() -> (bool, bool) {
    let output = Command::new("ufw")
        .arg("status")
        .arg("verbose")
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).to_lowercase();
            let is_active = s.contains("status: active");
            let default_deny = s.contains("default: deny (incoming)") || s.contains("default: reject (incoming)");
            return (is_active, default_deny);
        }
    }

    // Secondary check: iptables rules or systemctl is-active
    let svc = Command::new("systemctl")
        .args(["is-active", "ufw"])
        .output();
    if let Ok(s_out) = svc {
        let st = String::from_utf8_lossy(&s_out.stdout).trim().to_string();
        if st == "active" {
            return (true, true);
        }
    }

    (false, false)
}

fn read_sysctl_i32(key: &str) -> Option<i32> {
    let out = Command::new("sysctl")
        .arg("-n")
        .arg(key)
        .output()
        .ok()?;

    if out.status.success() {
        let val_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
        val_str.parse().ok()
    } else {
        None
    }
}

fn read_screen_idle_delay() -> u32 {
    let out = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.session", "idle-delay"])
        .output();

    if let Ok(o) = out {
        if o.status.success() {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            // Output format: "uint32 900" or "900"
            let num_str: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = num_str.parse() {
                return n;
            }
        }
    }
    900
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_posture_score_computation() {
        let report = PostureScoreReport::compute();
        assert!(report.total_score <= 100);
        assert!(!report.letter_grade.is_empty());
    }
}
