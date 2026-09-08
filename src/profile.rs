use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    Home,
    Work,
    Dev,
    Secure,
}

impl Profile {
    pub fn as_str(&self) -> &'static str {
        match self {
            Profile::Home => "home",
            Profile::Work => "work",
            Profile::Dev => "dev",
            Profile::Secure => "secure",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Profile::Home => "Home / Streaming / Gaming",
            Profile::Work => "Work / Office / Corporate VPN",
            Profile::Dev => "Developer / Coding Mode",
            Profile::Secure => "Hardened Travel / Lockdown",
        }
    }

    #[allow(dead_code)]
    pub fn icon_name(&self) -> &'static str {
        match self {
            Profile::Home => "user-home-symbolic",
            Profile::Work => "work-symbolic",
            Profile::Dev => "applications-development-symbolic",
            Profile::Secure => "security-high-symbolic",
        }
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Profile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "home" => Ok(Profile::Home),
            "work" => Ok(Profile::Work),
            "dev" => Ok(Profile::Dev),
            "secure" => Ok(Profile::Secure),
            other => Err(format!("Unknown profile: '{}'. Expected home, work, dev, or secure.", other)),
        }
    }
}
