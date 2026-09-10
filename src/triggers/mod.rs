use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub const SYSTEM_TRIGGERS_FILE: &str = "/etc/postureflow/triggers.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TriggerRule {
    pub name: String,
    pub process_names: Vec<String>,
    #[serde(default)]
    pub target_profile: Option<String>,
    #[serde(default)]
    pub boost_cpu_epp: Option<String>,
    #[serde(default)]
    pub boost_sysctl: HashMap<String, String>,
    #[serde(default)]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TriggersConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_check_interval")]
    pub check_interval_seconds: u64,
    #[serde(default)]
    pub rules: Vec<TriggerRule>,
}

fn default_true() -> bool {
    true
}

fn default_check_interval() -> u64 {
    3
}

impl Default for TriggersConfig {
    fn default() -> Self {
        default_triggers_config()
    }
}

pub fn default_triggers_config() -> TriggersConfig {
    let mut steam_sysctl = HashMap::new();
    steam_sysctl.insert("vm.max_map_count".to_string(), "2147483642".to_string());

    let mut dev_sysctl = HashMap::new();
    dev_sysctl.insert("fs.inotify.max_user_watches".to_string(), "1048576".to_string());

    let rules = vec![
        TriggerRule {
            name: "Gaming & Steam Engine Boost".to_string(),
            process_names: vec![
                "steam".to_string(),
                "steamwebhelper".to_string(),
                "gamescope".to_string(),
                "lutris".to_string(),
                "heroic".to_string(),
                "wine-preloader".to_string(),
                "wine64-preloader".to_string(),
                "proton".to_string(),
            ],
            target_profile: Some("home".to_string()),
            boost_cpu_epp: Some("performance".to_string()),
            boost_sysctl: steam_sysctl,
            comment: Some("Maximum CPU responsiveness and 2.1B map count for Steam & Proton gaming".to_string()),
        },
        TriggerRule {
            name: "Dev Containers & Coding Workload".to_string(),
            process_names: vec![
                "dockerd".to_string(),
                "containerd".to_string(),
                "podman".to_string(),
                "cargo".to_string(),
                "rust-analyzer".to_string(),
                "code".to_string(),
            ],
            target_profile: Some("dev".to_string()),
            boost_cpu_epp: Some("performance".to_string()),
            boost_sysctl: dev_sysctl,
            comment: Some("Auto-transition to Dev mode and boost inotify limits when coding".to_string()),
        },
        TriggerRule {
            name: "Office & Meetings Suite".to_string(),
            process_names: vec![
                "slack".to_string(),
                "teams".to_string(),
                "zoom".to_string(),
                "thunderbird".to_string(),
            ],
            target_profile: Some("work".to_string()),
            boost_cpu_epp: Some("balance_power".to_string()),
            boost_sysctl: HashMap::new(),
            comment: Some("Quiet fan acoustics and work VPN/CUPS access during office work".to_string()),
        },
    ];

    TriggersConfig {
        enabled: true,
        check_interval_seconds: 3,
        rules,
    }
}

pub fn triggers_config_path() -> PathBuf {
    if let Ok(p) = std::env::var("POSTUREFLOW_TRIGGERS_FILE") {
        return PathBuf::from(p);
    }
    if let Ok(cfg_home) = std::env::var("XDG_CONFIG_HOME") {
        let p = PathBuf::from(cfg_home).join("postureflow/triggers.toml");
        if p.exists() {
            return p;
        }
    } else if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join(".config/postureflow/triggers.toml");
        if p.exists() {
            return p;
        }
    }
    PathBuf::from(SYSTEM_TRIGGERS_FILE)
}

pub fn load_triggers_config() -> TriggersConfig {
    let path = triggers_config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cfg) = toml::from_str::<TriggersConfig>(&content) {
                return cfg;
            }
        }
    }
    default_triggers_config()
}

pub fn save_triggers_config(config: &TriggersConfig) -> Result<PathBuf, String> {
    let target = triggers_config_path();
    if let Some(parent) = target.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let serialized = toml::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize triggers config: {}", e))?;
    fs::write(&target, serialized)
        .map_err(|e| format!("Failed to write triggers config to {}: {}", target.display(), e))?;
    Ok(target)
}

/// Scans procfs `/proc/<pid>/comm` in memory with sub-millisecond overhead
pub fn scan_running_processes() -> HashSet<String> {
    let mut procs = HashSet::new();
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.chars().all(|c| c.is_ascii_digit()) {
                let comm_path = entry.path().join("comm");
                if let Ok(comm) = fs::read_to_string(&comm_path) {
                    let clean = comm.trim().to_lowercase();
                    if !clean.is_empty() {
                        procs.insert(clean);
                    }
                }
            }
        }
    }
    procs
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedTrigger {
    pub rule_name: String,
    pub matched_processes: Vec<String>,
    pub target_profile: Option<String>,
    pub boost_cpu_epp: Option<String>,
    pub boost_sysctl: HashMap<String, String>,
}

pub fn evaluate_triggers(
    config: &TriggersConfig,
    running: &HashSet<String>,
) -> Option<MatchedTrigger> {
    if !config.enabled {
        return None;
    }

    for rule in &config.rules {
        let mut matched = Vec::new();
        for p in &rule.process_names {
            let p_lower = p.to_lowercase();
            if running.contains(&p_lower) {
                matched.push(p.clone());
            }
        }
        if !matched.is_empty() {
            return Some(MatchedTrigger {
                rule_name: rule.name.clone(),
                matched_processes: matched,
                target_profile: rule.target_profile.clone(),
                boost_cpu_epp: rule.boost_cpu_epp.clone(),
                boost_sysctl: rule.boost_sysctl.clone(),
            });
        }
    }
    None
}

#[derive(Debug, Clone, Default)]
pub struct TriggerSession {
    pub active_rule_name: Option<String>,
    pub baseline_profile: Option<String>,
    pub baseline_cpu_epp: Option<String>,
    pub baseline_sysctls: HashMap<String, String>,
}

pub fn read_sysctl_value(key: &str) -> Option<String> {
    Command::new("sysctl")
        .args(["-n", key])
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })
}

pub fn apply_sysctl_override(key: &str, val: &str) -> Result<(), String> {
    let output = Command::new("sysctl")
        .args(["-w", &format!("{}={}", key, val)])
        .output()
        .map_err(|e| format!("Failed to apply sysctl {}={}: {}", key, val, e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_integrity() {
        let cfg = default_triggers_config();
        assert!(cfg.enabled);
        assert_eq!(cfg.rules.len(), 3);
        assert!(cfg.rules.iter().any(|r| r.name.contains("Gaming")));
    }

    #[test]
    fn test_rule_matching_steam() {
        let cfg = default_triggers_config();
        let mut running = HashSet::new();
        running.insert("bash".to_string());
        running.insert("steam".to_string());

        let matched = evaluate_triggers(&cfg, &running);
        assert!(matched.is_some());
        let m = matched.unwrap();
        assert!(m.rule_name.contains("Gaming"));
        assert_eq!(m.matched_processes, vec!["steam"]);
        assert_eq!(m.target_profile.as_deref(), Some("home"));
        assert_eq!(m.boost_cpu_epp.as_deref(), Some("performance"));
        assert_eq!(m.boost_sysctl.get("vm.max_map_count").unwrap(), "2147483642");
    }

    #[test]
    fn test_rule_matching_dev() {
        let cfg = default_triggers_config();
        let mut running = HashSet::new();
        running.insert("cargo".to_string());

        let matched = evaluate_triggers(&cfg, &running);
        assert!(matched.is_some());
        let m = matched.unwrap();
        assert!(m.rule_name.contains("Coding"));
        assert_eq!(m.matched_processes, vec!["cargo"]);
        assert_eq!(m.target_profile.as_deref(), Some("dev"));
    }

    #[test]
    fn test_disabled_triggers_return_none() {
        let mut cfg = default_triggers_config();
        cfg.enabled = false;
        let mut running = HashSet::new();
        running.insert("steam".to_string());

        assert!(evaluate_triggers(&cfg, &running).is_none());
    }

    #[test]
    fn test_toml_serialization() {
        let cfg = default_triggers_config();
        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        let parsed: TriggersConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(cfg, parsed);
    }
}
