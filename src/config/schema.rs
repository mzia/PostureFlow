use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileMetadata {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub is_builtin: bool,
}

fn default_icon() -> String {
    "preferences-system-symbolic".to_string()
}

fn default_version() -> String {
    "1.0.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SecurityLimitsConfig {
    #[serde(default)]
    pub core_dump: Option<String>,
    #[serde(default)]
    pub nofile_soft: Option<u64>,
    #[serde(default)]
    pub nofile_hard: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PortRule {
    pub port: String,
    #[serde(default)]
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FirewallConfig {
    #[serde(default = "default_deny")]
    pub default_incoming: String,
    #[serde(default = "default_allow")]
    pub default_outgoing: String,
    #[serde(default = "default_true")]
    pub allow_loopback: bool,
    #[serde(default)]
    pub allow_interfaces: Vec<String>,
    #[serde(default)]
    pub allow_ports: Vec<PortRule>,
}

fn default_deny() -> String {
    "deny".to_string()
}

fn default_allow() -> String {
    "allow".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for FirewallConfig {
    fn default() -> Self {
        Self {
            default_incoming: default_deny(),
            default_outgoing: default_allow(),
            allow_loopback: true,
            allow_interfaces: Vec::new(),
            allow_ports: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FrameworkPowerConfig {
    #[serde(default)]
    pub battery_charge_limit: Option<u32>,
    #[serde(default)]
    pub power_profile: Option<String>,
    #[serde(default)]
    pub cpu_epp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PeripheralsConfig {
    #[serde(default)]
    pub block_new_usb: Option<bool>,
    #[serde(default)]
    pub bluetooth: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DesktopConfig {
    #[serde(default)]
    pub idle_delay_seconds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileConfig {
    pub profile: ProfileMetadata,
    #[serde(default)]
    pub kernel: HashMap<String, String>,
    #[serde(default)]
    pub limits: SecurityLimitsConfig,
    #[serde(default)]
    pub firewall: FirewallConfig,
    #[serde(default)]
    pub power: FrameworkPowerConfig,
    #[serde(default)]
    pub peripherals: PeripheralsConfig,
    #[serde(default)]
    pub desktop: DesktopConfig,
}

impl ProfileConfig {
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }
}
