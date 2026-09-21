use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const SYSTEM_THEME_CONFIG: &str = "/etc/postureflow/theme.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileTheme {
    pub display_name: String,
    pub accent_hex: String,
    pub wallpaper_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DynamicThemeConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub apply_accent: bool,
    #[serde(default = "default_true")]
    pub apply_wallpaper: bool,
    #[serde(default = "default_themes")]
    pub themes: HashMap<String, ProfileTheme>,
}

fn default_true() -> bool {
    true
}

fn default_themes() -> HashMap<String, ProfileTheme> {
    let mut map = HashMap::new();

    map.insert(
        "home".to_string(),
        ProfileTheme {
            display_name: "Warm Amber Sunset".to_string(),
            accent_hex: "#FF8C00FF".to_string(),
            wallpaper_path: "/usr/share/backgrounds/cosmic/A_stormy_stellar_nursery_esa_379309.jpg".to_string(),
        },
    );

    map.insert(
        "work".to_string(),
        ProfileTheme {
            display_name: "Focused Corporate Cyan".to_string(),
            accent_hex: "#48B9C7FF".to_string(),
            wallpaper_path: "/usr/share/backgrounds/cosmic/COSMIC-logo-Dark.png".to_string(),
        },
    );

    map.insert(
        "dev".to_string(),
        ProfileTheme {
            display_name: "Matrix Hacker Emerald".to_string(),
            accent_hex: "#26AC72FF".to_string(),
            wallpaper_path: "/usr/share/backgrounds/cosmic/COSMIC-Pattern-Dark.png".to_string(),
        },
    );

    map.insert(
        "travel".to_string(),
        ProfileTheme {
            display_name: "Stealth Crimson Lockdown".to_string(),
            accent_hex: "#FF5555FF".to_string(),
            wallpaper_path: "/usr/share/backgrounds/cosmic/orion_nebula_nasa_heic0601a.jpg".to_string(),
        },
    );

    map
}

impl Default for DynamicThemeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            apply_accent: true,
            apply_wallpaper: true,
            themes: default_themes(),
        }
    }
}

impl DynamicThemeConfig {
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }
}

pub fn user_theme_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config/postureflow/theme.toml")
}

pub fn load_theme_config() -> DynamicThemeConfig {
    let user_path = user_theme_config_path();
    if user_path.exists() {
        if let Ok(content) = fs::read_to_string(&user_path) {
            if let Ok(cfg) = DynamicThemeConfig::from_toml(&content) {
                return cfg;
            }
        }
    }

    let sys_path = Path::new(SYSTEM_THEME_CONFIG);
    if sys_path.exists() {
        if let Ok(content) = fs::read_to_string(sys_path) {
            if let Ok(cfg) = DynamicThemeConfig::from_toml(&content) {
                return cfg;
            }
        }
    }

    DynamicThemeConfig::default()
}

pub fn save_theme_config(config: &DynamicThemeConfig, privileged: bool) -> Result<(), String> {
    let toml = config.to_toml().map_err(|e| format!("Serialization error: {}", e))?;

    let path = if privileged {
        PathBuf::from(SYSTEM_THEME_CONFIG)
    } else {
        user_theme_config_path()
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    fs::write(&path, toml).map_err(|e| format!("Failed to write {:?}: {}", path, e))?;
    Ok(())
}

/// Apply posture-aware ambient theme (wallpaper + accent) to COSMIC and GNOME
pub fn apply_posture_theme(profile_id: &str) -> Result<(), String> {
    let config = load_theme_config();
    if !config.enabled {
        return Ok(());
    }

    if let Some(theme) = config.themes.get(profile_id) {
        println!(
            "[+] 🎨 Applying dynamic posture theme for [{}]: {}",
            profile_id.to_uppercase(),
            theme.display_name
        );

        if config.apply_accent {
            let _ = apply_cosmic_accent(&theme.accent_hex);
        }

        if config.apply_wallpaper && Path::new(&theme.wallpaper_path).exists() {
            let _ = apply_cosmic_wallpaper(&theme.wallpaper_path);
            let _ = apply_gnome_wallpaper(&theme.wallpaper_path);
        }
    }

    Ok(())
}

fn apply_cosmic_accent(accent_hex: &str) -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let content = format!("Some(\"{}\")\n", accent_hex);

    let targets = [
        format!("{}/.config/cosmic/com.system76.CosmicTheme.Dark.Builder/v2/accent", home),
        format!("{}/.config/cosmic/com.system76.CosmicTheme.Dark/v2/accent", home),
    ];

    for t in &targets {
        let p = Path::new(t);
        if let Some(parent) = p.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(p, &content);
    }

    Ok(())
}

fn apply_cosmic_wallpaper(wallpaper_path: &str) -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let bg_file = format!("{}/.config/cosmic/com.system76.CosmicBackground/v1/all", home);

    let ron_content = format!(
        "(\n    output: \"all\",\n    source: Path(\"{}\"),\n    filter_by_theme: true,\n    rotation_frequency: 300,\n    filter_method: Lanczos,\n    scaling_mode: Zoom,\n    sampling_method: Alphanumeric,\n)\n",
        wallpaper_path
    );

    let p = Path::new(&bg_file);
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(p, ron_content).map_err(|e| format!("Failed to update COSMIC wallpaper: {}", e))?;

    Ok(())
}

fn apply_gnome_wallpaper(wallpaper_path: &str) {
    let uri = format!("file://{}", wallpaper_path);
    let _ = Command::new("gsettings")
        .args(["set", "org.gnome.desktop.background", "picture-uri-dark", &uri])
        .output();
    let _ = Command::new("gsettings")
        .args(["set", "org.gnome.desktop.background", "picture-uri", &uri])
        .output();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_theme_config() {
        let cfg = DynamicThemeConfig::default();
        assert!(cfg.enabled);
        assert!(cfg.themes.contains_key("home"));
        assert!(cfg.themes.contains_key("work"));
        assert!(cfg.themes.contains_key("dev"));
        assert!(cfg.themes.contains_key("travel"));
    }

    #[test]
    fn test_theme_toml_roundtrip() {
        let cfg = DynamicThemeConfig::default();
        let toml = cfg.to_toml().unwrap();
        let loaded = DynamicThemeConfig::from_toml(&toml).unwrap();
        assert_eq!(cfg, loaded);
    }
}
