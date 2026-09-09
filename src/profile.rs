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
    pub const ALL: [Profile; 4] = [
        Profile::Home,
        Profile::Work,
        Profile::Dev,
        Profile::Secure,
    ];

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

    pub fn icon_name(&self) -> &'static str {
        match self {
            Profile::Home => "user-home-symbolic",
            Profile::Work => "applications-office-symbolic",
            Profile::Dev => "utilities-terminal-symbolic",
            Profile::Secure => "security-high-symbolic",
        }
    }

    pub fn next(&self) -> Profile {
        match self {
            Profile::Home => Profile::Work,
            Profile::Work => Profile::Dev,
            Profile::Dev => Profile::Secure,
            Profile::Secure => Profile::Home,
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
            "home" | "default" => Ok(Profile::Home),
            "work" => Ok(Profile::Work),
            "dev" => Ok(Profile::Dev),
            "secure" => Ok(Profile::Secure),
            other => Err(format!("Unknown profile: '{}'. Expected home, work, dev, or secure.", other)),
        }
    }
}
