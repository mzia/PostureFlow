use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const SYSTEM_SCHEDULE_FILE: &str = "/etc/postureflow/schedule.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleWindow {
    pub name: String,
    #[serde(default)]
    pub days: Vec<String>,
    pub start_time: String,
    pub end_time: String,
    pub target_profile: String,
    #[serde(default)]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatteryEmergencyConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_threshold")]
    pub threshold_percent: u32,
    #[serde(default = "default_emergency_profile")]
    pub target_profile: String,
    #[serde(default = "default_power_epp")]
    pub force_cpu_epp: Option<String>,
    #[serde(default = "default_true")]
    pub disable_bluetooth: bool,
    #[serde(default = "default_true")]
    pub auto_recover_on_ac: bool,
}

fn default_true() -> bool {
    true
}

fn default_threshold() -> u32 {
    15
}

fn default_emergency_profile() -> String {
    "travel".to_string()
}

fn default_power_epp() -> Option<String> {
    Some("power".to_string())
}

impl Default for BatteryEmergencyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold_percent: 15,
            target_profile: "travel".to_string(),
            force_cpu_epp: Some("power".to_string()),
            disable_bluetooth: true,
            auto_recover_on_ac: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CircadianConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_interval")]
    pub check_interval_seconds: u64,
    #[serde(default)]
    pub battery_emergency: BatteryEmergencyConfig,
    #[serde(default)]
    pub schedules: Vec<ScheduleWindow>,
}

fn default_interval() -> u64 {
    10
}

impl Default for CircadianConfig {
    fn default() -> Self {
        default_schedule_config()
    }
}

pub fn default_schedule_config() -> CircadianConfig {
    CircadianConfig {
        enabled: true,
        check_interval_seconds: 10,
        battery_emergency: BatteryEmergencyConfig::default(),
        schedules: vec![
            ScheduleWindow {
                name: "Workday Office Hours".to_string(),
                days: vec!["Mon".into(), "Tue".into(), "Wed".into(), "Thu".into(), "Fri".into()],
                start_time: "09:00".to_string(),
                end_time: "17:00".to_string(),
                target_profile: "work".to_string(),
                comment: Some("Work & Corporate VPN posture during office hours".to_string()),
            },
            ScheduleWindow {
                name: "Evening Wind-Down & Gaming".to_string(),
                days: vec!["Mon".into(), "Tue".into(), "Wed".into(), "Thu".into(), "Fri".into(), "Sat".into(), "Sun".into()],
                start_time: "17:00".to_string(),
                end_time: "23:00".to_string(),
                target_profile: "home".to_string(),
                comment: Some("Streaming, Proton gaming, and media mode".to_string()),
            },
            ScheduleWindow {
                name: "Overnight Stealth Lockdown".to_string(),
                days: vec!["Mon".into(), "Tue".into(), "Wed".into(), "Thu".into(), "Fri".into(), "Sat".into(), "Sun".into()],
                start_time: "23:00".to_string(),
                end_time: "07:00".to_string(),
                target_profile: "travel".to_string(),
                comment: Some("Maximum firewall stealth and perimeter defense overnight".to_string()),
            },
        ],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeOfDay {
    pub hour: u32,
    pub minute: u32,
}

impl TimeOfDay {
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.trim().split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        let h: u32 = parts[0].parse().ok()?;
        let m: u32 = parts[1].parse().ok()?;
        if h < 24 && m < 60 {
            Some(TimeOfDay { hour: h, minute: m })
        } else {
            None
        }
    }

    pub fn to_minutes(&self) -> u32 {
        self.hour * 60 + self.minute
    }
}

pub fn is_time_in_range(time: TimeOfDay, start: TimeOfDay, end: TimeOfDay) -> bool {
    let t = time.to_minutes();
    let s = start.to_minutes();
    let e = end.to_minutes();
    if s <= e {
        // Normal range within same calendar day (e.g. 09:00 - 17:00)
        t >= s && t < e
    } else {
        // Crossover midnight range (e.g. 23:00 - 07:00)
        t >= s || t < e
    }
}

pub fn matches_weekday(wday: u32, rule_days: &[String]) -> bool {
    if rule_days.is_empty() {
        return true;
    }

    for day_str in rule_days {
        let clean = day_str.trim().to_lowercase();
        if clean == "*" || clean == "all" || clean == "daily" {
            return true;
        }
        if clean == "weekdays" || clean == "workdays" {
            if (1..=5).contains(&wday) {
                return true;
            }
        }
        if clean == "weekends" {
            if wday == 0 || wday == 6 {
                return true;
            }
        }

        let target_wday = match clean.as_str() {
            "sun" | "sunday" => Some(0),
            "mon" | "monday" => Some(1),
            "tue" | "tuesday" => Some(2),
            "wed" | "wednesday" => Some(3),
            "thu" | "thursday" => Some(4),
            "fri" | "friday" => Some(5),
            "sat" | "saturday" => Some(6),
            _ => None,
        };

        if let Some(target) = target_wday {
            if target == wday {
                return true;
            }
        }
    }

    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    pub hour: u32,
    pub minute: u32,
    pub weekday: u32, // 0 = Sun, 1 = Mon, ..., 6 = Sat
}

impl LocalTime {
    pub fn weekday_name(&self) -> &'static str {
        match self.weekday {
            0 => "Sunday",
            1 => "Monday",
            2 => "Tuesday",
            3 => "Wednesday",
            4 => "Thursday",
            5 => "Friday",
            6 => "Saturday",
            _ => "Unknown",
        }
    }
}

pub fn get_current_local_time() -> LocalTime {
    unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        LocalTime {
            hour: tm.tm_hour as u32,
            minute: tm.tm_min as u32,
            weekday: tm.tm_wday as u32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatteryInfo {
    pub capacity_percent: Option<u32>,
    pub status: String,
    pub is_discharging: bool,
    pub on_ac_power: bool,
}

pub fn get_battery_status() -> BatteryInfo {
    let mut capacity = None;
    let mut status = "Unknown".to_string();
    let mut is_discharging = false;
    let mut on_ac_power = false;

    let sys_power = PathBuf::from("/sys/class/power_supply");
    if let Ok(entries) = fs::read_dir(&sys_power) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Check AC Adapter / Mains
            let type_path = path.join("type");
            let type_str = fs::read_to_string(&type_path).unwrap_or_default().trim().to_string();

            if type_str.eq_ignore_ascii_case("Mains") || name.starts_with("AC") {
                if let Ok(online_val) = fs::read_to_string(path.join("online")) {
                    if online_val.trim() == "1" {
                        on_ac_power = true;
                    }
                }
            }

            // Check Battery (prioritizing primary system battery BAT0, BAT1, etc.)
            let scope_str = fs::read_to_string(path.join("scope")).unwrap_or_default().trim().to_string();
            let is_peripheral = scope_str.eq_ignore_ascii_case("Device") || name.starts_with("hid");

            if (type_str.eq_ignore_ascii_case("Battery") || name.starts_with("BAT")) && !is_peripheral {
                let is_primary_name = name.starts_with("BAT");
                if capacity.is_none() || is_primary_name {
                    if let Ok(cap_str) = fs::read_to_string(path.join("capacity")) {
                        if let Ok(cap) = cap_str.trim().parse::<u32>() {
                            capacity = Some(cap);
                        }
                    }
                    if let Ok(st_str) = fs::read_to_string(path.join("status")) {
                        let clean_st = st_str.trim().to_string();
                        is_discharging = clean_st.eq_ignore_ascii_case("Discharging");
                        status = clean_st;
                    }
                }
            }
        }
    }

    BatteryInfo {
        capacity_percent: capacity,
        status,
        is_discharging,
        on_ac_power,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleDecision {
    BatteryEmergency {
        rule_name: String,
        target_profile: String,
        battery_percent: u32,
        cpu_epp: Option<String>,
        disable_bluetooth: bool,
    },
    ScheduledShift {
        rule_name: String,
        target_profile: String,
        window: String,
    },
}

pub fn evaluate_schedule(
    cfg: &CircadianConfig,
    now: &LocalTime,
    battery: &BatteryInfo,
) -> Option<ScheduleDecision> {
    if !cfg.enabled {
        return None;
    }

    // 1. Highest Priority: Battery Emergency Fallback
    if cfg.battery_emergency.enabled {
        if let Some(cap) = battery.capacity_percent {
            if battery.is_discharging && cap <= cfg.battery_emergency.threshold_percent {
                return Some(ScheduleDecision::BatteryEmergency {
                    rule_name: "Battery Critical Fallback".to_string(),
                    target_profile: cfg.battery_emergency.target_profile.clone(),
                    battery_percent: cap,
                    cpu_epp: cfg.battery_emergency.force_cpu_epp.clone(),
                    disable_bluetooth: cfg.battery_emergency.disable_bluetooth,
                });
            }
        }
    }

    // 2. Normal Time-of-Day Schedule Windows
    let tod = TimeOfDay { hour: now.hour, minute: now.minute };
    for rule in &cfg.schedules {
        if matches_weekday(now.weekday, &rule.days) {
            if let (Some(start), Some(end)) = (TimeOfDay::parse(&rule.start_time), TimeOfDay::parse(&rule.end_time)) {
                if is_time_in_range(tod, start, end) {
                    return Some(ScheduleDecision::ScheduledShift {
                        rule_name: rule.name.clone(),
                        target_profile: rule.target_profile.clone(),
                        window: format!("{}-{}", rule.start_time, rule.end_time),
                    });
                }
            }
        }
    }

    None
}

pub fn primary_config_path() -> PathBuf {
    PathBuf::from(SYSTEM_SCHEDULE_FILE)
}

pub fn user_config_path() -> Option<PathBuf> {
    dirs_next_config_dir().map(|p| p.join("postureflow").join("schedule.toml"))
}

fn dirs_next_config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
}

pub fn load_schedule_config() -> CircadianConfig {
    // 1. Check user override
    if let Some(user_path) = user_config_path() {
        if user_path.exists() {
            if let Ok(content) = fs::read_to_string(&user_path) {
                if let Ok(cfg) = toml::from_str::<CircadianConfig>(&content) {
                    return cfg;
                }
            }
        }
    }

    // 2. Check system configuration
    let sys_path = primary_config_path();
    if sys_path.exists() {
        if let Ok(content) = fs::read_to_string(&sys_path) {
            if let Ok(cfg) = toml::from_str::<CircadianConfig>(&content) {
                return cfg;
            }
        }
    }

    // 3. Fallback default
    default_schedule_config()
}

pub fn save_schedule_config(cfg: &CircadianConfig) -> Result<(), String> {
    let toml_str = toml::to_string_pretty(cfg)
        .map_err(|e| format!("Failed to serialize schedule config: {}", e))?;

    let path = primary_config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    fs::write(&path, toml_str)
        .map_err(|e| format!("Failed to write schedule config to {:?}: {}", path, e))?;

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_default_config_integrity() {
        let cfg = default_schedule_config();
        assert!(cfg.enabled);
        assert_eq!(cfg.check_interval_seconds, 10);
        assert_eq!(cfg.battery_emergency.threshold_percent, 15);
        assert_eq!(cfg.schedules.len(), 3);
    }

    #[test]
    fn test_time_of_day_parsing() {
        let t1 = TimeOfDay::parse("09:00").unwrap();
        assert_eq!(t1.hour, 9);
        assert_eq!(t1.minute, 0);

        let t2 = TimeOfDay::parse("23:59").unwrap();
        assert_eq!(t2.hour, 23);
        assert_eq!(t2.minute, 59);

        assert!(TimeOfDay::parse("24:00").is_none());
        assert!(TimeOfDay::parse("09:60").is_none());
        assert!(TimeOfDay::parse("invalid").is_none());
    }

    #[test]
    fn test_time_in_range_standard() {
        let start = TimeOfDay::parse("09:00").unwrap();
        let end = TimeOfDay::parse("17:00").unwrap();

        assert!(is_time_in_range(TimeOfDay::parse("09:00").unwrap(), start, end));
        assert!(is_time_in_range(TimeOfDay::parse("12:30").unwrap(), start, end));
        assert!(is_time_in_range(TimeOfDay::parse("16:59").unwrap(), start, end));

        assert!(!is_time_in_range(TimeOfDay::parse("08:59").unwrap(), start, end));
        assert!(!is_time_in_range(TimeOfDay::parse("17:00").unwrap(), start, end));
        assert!(!is_time_in_range(TimeOfDay::parse("20:00").unwrap(), start, end));
    }

    #[test]
    fn test_time_in_range_midnight_crossover() {
        let start = TimeOfDay::parse("23:00").unwrap();
        let end = TimeOfDay::parse("07:00").unwrap();

        assert!(is_time_in_range(TimeOfDay::parse("23:00").unwrap(), start, end));
        assert!(is_time_in_range(TimeOfDay::parse("23:45").unwrap(), start, end));
        assert!(is_time_in_range(TimeOfDay::parse("00:00").unwrap(), start, end));
        assert!(is_time_in_range(TimeOfDay::parse("03:30").unwrap(), start, end));
        assert!(is_time_in_range(TimeOfDay::parse("06:59").unwrap(), start, end));

        assert!(!is_time_in_range(TimeOfDay::parse("07:00").unwrap(), start, end));
        assert!(!is_time_in_range(TimeOfDay::parse("12:00").unwrap(), start, end));
        assert!(!is_time_in_range(TimeOfDay::parse("22:59").unwrap(), start, end));
    }

    #[test]
    fn test_matches_weekday() {
        let weekdays = vec!["Mon".to_string(), "Tue".to_string(), "Wed".to_string(), "Thu".to_string(), "Fri".to_string()];
        assert!(matches_weekday(1, &weekdays)); // Monday
        assert!(matches_weekday(5, &weekdays)); // Friday
        assert!(!matches_weekday(0, &weekdays)); // Sunday
        assert!(!matches_weekday(6, &weekdays)); // Saturday

        let weekends = vec!["Weekends".to_string()];
        assert!(matches_weekday(0, &weekends));
        assert!(matches_weekday(6, &weekends));
        assert!(!matches_weekday(2, &weekends));

        let any_day = vec!["*".to_string()];
        assert!(matches_weekday(3, &any_day));
    }

    #[test]
    fn test_battery_emergency_trigger() {
        let cfg = default_schedule_config();
        let now = LocalTime { hour: 12, minute: 0, weekday: 1 };
        let battery_discharging_low = BatteryInfo {
            capacity_percent: Some(10),
            status: "Discharging".to_string(),
            is_discharging: true,
            on_ac_power: false,
        };

        let decision = evaluate_schedule(&cfg, &now, &battery_discharging_low);
        match decision {
            Some(ScheduleDecision::BatteryEmergency { battery_percent, target_profile, .. }) => {
                assert_eq!(battery_percent, 10);
                assert_eq!(target_profile, "travel");
            }
            _ => panic!("Expected BatteryEmergency decision"),
        }
    }

    #[test]
    fn test_battery_emergency_suppressed_on_ac() {
        let cfg = default_schedule_config();
        let now = LocalTime { hour: 12, minute: 0, weekday: 1 };
        let battery_charging_low = BatteryInfo {
            capacity_percent: Some(10),
            status: "Charging".to_string(),
            is_discharging: false,
            on_ac_power: true,
        };

        let decision = evaluate_schedule(&cfg, &now, &battery_charging_low);
        match decision {
            Some(ScheduleDecision::ScheduledShift { target_profile, .. }) => {
                // At 12:00 on Monday, workday office hours matches "work"
                assert_eq!(target_profile, "work");
            }
            _ => panic!("Expected ScheduledShift to 'work' profile, got: {:?}", decision),
        }
    }

    #[test]
    fn test_toml_serialization() {
        let cfg = default_schedule_config();
        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        let parsed: CircadianConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(cfg, parsed);
    }
}
