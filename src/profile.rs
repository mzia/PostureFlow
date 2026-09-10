use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    Home,
    Work,
    Dev,
    #[serde(alias = "secure")]
    Travel,
}

#[allow(dead_code)]
impl Profile {
    pub const ALL: [Profile; 4] = [
        Profile::Home,
        Profile::Work,
        Profile::Dev,
        Profile::Travel,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Profile::Home => "home",
            Profile::Work => "work",
            Profile::Dev => "dev",
            Profile::Travel => "travel",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Profile::Home => "Home / Streaming / Gaming",
            Profile::Work => "Work / Office / Corporate VPN",
            Profile::Dev => "Developer / Coding Mode",
            Profile::Travel => "Travel / Public Wi-Fi Lockdown",
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            Profile::Home => "postureflow-home-symbolic",
            Profile::Work => "postureflow-work-symbolic",
            Profile::Dev => "postureflow-dev-symbolic",
            Profile::Travel => "postureflow-travel-symbolic",
        }
    }

    pub fn fallback_icon_name(&self) -> &'static str {
        match self {
            Profile::Home => "user-home-symbolic",
            Profile::Work => "applications-office-symbolic",
            Profile::Dev => "utilities-terminal-symbolic",
            Profile::Travel => "security-high-symbolic",
        }
    }

    pub fn next(&self) -> Profile {
        match self {
            Profile::Home => Profile::Work,
            Profile::Work => Profile::Dev,
            Profile::Dev => Profile::Travel,
            Profile::Travel => Profile::Home,
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
            "home" | "default" | "" => Ok(Profile::Home),
            "work" => Ok(Profile::Work),
            "dev" => Ok(Profile::Dev),
            "travel" | "secure" => Ok(Profile::Travel),
            other => Err(format!("Unknown profile: '{}'. Expected home, work, dev, or travel.", other)),
        }
    }
}
