use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use clap::Parser;
use postureflow::profile::Profile;
use postureflow::dbus::{PostureFlowService, DBUS_INTERFACE, DBUS_PATH};
use postureflow::system;
use postureflow::inspector::{ports, PostureScoreReport};
use postureflow::autoflow;
use postureflow::triggers;
use postureflow::schedule;

#[derive(Parser, Debug)]
#[command(name = "postureflow-daemon")]
#[command(author = "M. Zia")]
#[command(version = "1.0.0")]
#[command(about = "PostureFlow Context Posture Manager & D-Bus Daemon", long_about = None)]
struct Cli {
    /// Run as a background D-Bus service
    #[arg(long)]
    daemon: bool,

    /// Connect to Session Bus instead of System Bus (useful for testing)
    #[arg(long)]
    session_bus: bool,

    /// Activate Home / Streaming / Gaming mode
    #[arg(long)]
    home: bool,

    /// Activate Work / Office / Corporate VPN mode
    #[arg(long)]
    work: bool,

    /// Activate Developer / Coding mode
    #[arg(long, short)]
    dev: bool,

    /// Activate Hardened Travel / Lockdown mode
    #[arg(long, short = 't')]
    travel: bool,

    /// Legacy alias for --travel
    #[arg(long, short = 's')]
    secure: bool,

    /// Activate a specific profile by ID (built-in or custom)
    #[arg(long, short)]
    profile: Option<String>,

    /// Show current security posture report
    #[arg(long, short = 'i')]
    status: bool,

    /// Calculate and display the Real-Time Security Posture Score (0-100%)
    #[arg(long)]
    score: bool,

    /// Scan and display actively listening network ports and owning processes
    #[arg(long)]
    ports: bool,

    /// Display Auto-Flow network detection and configuration
    #[arg(long)]
    autoflow: bool,

    /// Display App-Aware Dynamic Triggers rules and running app status
    #[arg(long)]
    triggers: bool,

    /// Display Circadian Schedule and battery status report
    #[arg(long, short = 'c')]
    schedule: bool,

    /// Reset all settings to Pop!_OS factory defaults
    #[arg(long)]
    reset: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if cli.daemon {
        run_daemon(cli.session_bus).await?;
        return Ok(());
    }

    if cli.score {
        let report = PostureScoreReport::compute();
        println!("\n=== PostureFlow Security Cockpit ===");
        println!("Overall Score: {}/100 (Grade: {})", report.total_score, report.letter_grade);
        println!("\nBreakdown:");
        println!("  • Firewall Protection:   {}/35 pts (UFW: {})", report.firewall_score, if report.ufw_active { "Active" } else { "Inactive" });
        println!("  • Attack Surface:        {}/25 pts ({} public, {} local ports)", report.attack_surface_score, report.public_ports_count, report.local_ports_count);
        println!("  • Kernel Hardening:      {}/25 pts (ptrace: {}, bpf: {})", report.kernel_score, report.ptrace_scope, report.bpf_disabled);
        println!("  • Desktop & Limits:      {}/15 pts (Lock timeout: {}s)", report.limits_score, report.idle_timeout_seconds);

        if !report.recommendations.is_empty() {
            println!("\nRecommendations:");
            for r in &report.recommendations {
                println!("  [!] {}", r);
            }
        }
        println!();
        return Ok(());
    }

    if cli.ports {
        let port_list = ports::scan_listening_ports();
        println!("\n=== PostureFlow Listening Sockets Inspector ===");
        println!("{:<7} {:<22} {:<8} {:<10} {:<24} {:<8}", "PROTO", "ADDRESS:PORT", "PID", "PROCESS", "HINT", "EXPOSURE");
        println!("{:-<85}", "");
        for p in &port_list {
            let addr = format!("{}:{}", p.local_ip, p.port);
            let pid_str = p.pid.map(|n| n.to_string()).unwrap_or_else(|| "-".to_string());
            let exposure = if p.is_public { "EXPOSED TO LAN/WAN" } else { "LOCAL ONLY" };
            println!("{:<7} {:<22} {:<8} {:<10} {:<24} {:<8}", p.protocol.to_uppercase(), addr, pid_str, p.process_name, p.service_hint, exposure);
        }
        println!();
        return Ok(());
    }

    if cli.autoflow {
        let cfg = autoflow::load_autoflow_config();
        let net = autoflow::detect_active_networks();
        let matched = autoflow::evaluate_posture(&cfg, &net);

        println!("\n=== PostureFlow Auto-Flow Status ===");
        println!("Auto-Flow Enabled:       {}", if cfg.enabled { "YES" } else { "NO" });
        println!("Active Connection Type:  {}", net.primary_type);
        println!("Detected Wi-Fi SSID:     {}", net.current_ssid.as_deref().unwrap_or("None"));
        println!("Active VPN Tunnel:       {}", net.active_vpn.as_deref().unwrap_or("None"));
        println!("Active Devices:          {}", net.active_devices.join(", "));
        println!("Matched Profile:         {}", matched.as_deref().unwrap_or("No rule triggered").to_uppercase());
        println!("\nConfigured Rules ({}):", cfg.rules.len());
        for (i, r) in cfg.rules.iter().enumerate() {
            let trigger = if let Some(ref s) = r.ssid {
                format!("SSID '{}'", s)
            } else if let Some(ref iface) = r.interface {
                format!("Interface '{}'", iface)
            } else {
                "Unknown".to_string()
            };
            println!("  [{}] {} ➔ Profile: [{}] ({})", i + 1, trigger, r.profile.to_uppercase(), r.comment.as_deref().unwrap_or("-"));
        }
        println!();
        return Ok(());
    }

    if cli.triggers {
        let cfg = triggers::load_triggers_config();
        let running = triggers::scan_running_processes();
        let matched = triggers::evaluate_triggers(&cfg, &running);

        println!("\n=== PostureFlow App-Aware Dynamic Triggers ===");
        println!("Trigger Engine Enabled:   {}", if cfg.enabled { "YES" } else { "NO" });
        println!("Check Interval:           {} seconds", cfg.check_interval_seconds);
        if let Some(ref m) = matched {
            println!("Active Dynamic Trigger:   🚨 {} (Matched: {})", m.rule_name, m.matched_processes.join(", "));
            if let Some(ref p) = m.target_profile {
                println!("  • Target Posture:       [{}]", p.to_uppercase());
            }
            if let Some(ref epp) = m.boost_cpu_epp {
                println!("  • CPU EPP Boost:        {}", epp);
            }
            for (k, v) in &m.boost_sysctl {
                println!("  • Sysctl Boost:         {} = {}", k, v);
            }
        } else {
            println!("Active Dynamic Trigger:   None (Baseline profile active)");
        }

        println!("\nConfigured Application Rules ({}):", cfg.rules.len());
        for (i, r) in cfg.rules.iter().enumerate() {
            println!("  [{}] {}", i + 1, r.name);
            println!("      Watched Binaries:   {}", r.process_names.join(", "));
            if let Some(ref p) = r.target_profile {
                println!("      Target Profile:     [{}]", p.to_uppercase());
            }
            if let Some(ref epp) = r.boost_cpu_epp {
                println!("      CPU EPP Boost:      {}", epp);
            }
            for (k, v) in &r.boost_sysctl {
                println!("      Sysctl Boost:       {} = {}", k, v);
            }
            if let Some(ref c) = r.comment {
                println!("      Description:        {}", c);
            }
        }
        println!();
        return Ok(());
    }

    if cli.schedule {
        let cfg = schedule::load_schedule_config();
        let now = schedule::get_current_local_time();
        let battery = schedule::get_battery_status();
        let decision = schedule::evaluate_schedule(&cfg, &now, &battery);

        println!("\n=== PostureFlow Circadian & Scheduled Flow ===");
        println!("Schedule Engine Enabled:  {}", if cfg.enabled { "YES" } else { "NO" });
        println!("Local Time:               {:02}:{:02} ({})", now.hour, now.minute, now.weekday_name());
        println!("Battery Status:           {}% ({}, AC Online: {})",
            battery.capacity_percent.map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string()),
            battery.status,
            if battery.on_ac_power { "YES" } else { "NO" }
        );
        println!("Emergency Fallback (<{}%): Target [{}] (CPU EPP: {}, BT off: {})",
            cfg.battery_emergency.threshold_percent,
            cfg.battery_emergency.target_profile.to_uppercase(),
            cfg.battery_emergency.force_cpu_epp.as_deref().unwrap_or("default"),
            if cfg.battery_emergency.disable_bluetooth { "YES" } else { "NO" }
        );

        match decision {
            Some(schedule::ScheduleDecision::BatteryEmergency { ref rule_name, ref target_profile, battery_percent, .. }) => {
                println!("Active Scheduled State:   🚨 {} (Battery {}% <= {}%) ➔ [{}]",
                    rule_name, battery_percent, cfg.battery_emergency.threshold_percent, target_profile.to_uppercase());
            }
            Some(schedule::ScheduleDecision::ScheduledShift { ref rule_name, ref target_profile, ref window }) => {
                println!("Active Scheduled State:   ⏰ {} ({}) ➔ [{}]",
                    rule_name, window, target_profile.to_uppercase());
            }
            None => {
                println!("Active Scheduled State:   None (No window active / Manual profile holds)");
            }
        }

        println!("\nConfigured Schedule Windows ({}):", cfg.schedules.len());
        for (i, r) in cfg.schedules.iter().enumerate() {
            let days_str = if r.days.is_empty() { "*".to_string() } else { r.days.join(", ") };
            println!("  [{}] {} ({}) [{} - {}] ➔ Profile: [{}]",
                i + 1, r.name, days_str, r.start_time, r.end_time, r.target_profile.to_uppercase());
            if let Some(ref c) = r.comment {
                println!("      Description:        {}", c);
            }
        }
        println!();
        return Ok(());
    }

    if cli.status {
        println!("{}", system::get_status_report());
        return Ok(());
    }

    if cli.reset {
        println!("[*] Resetting firewall and sysctl to Pop!_OS factory defaults...");
        system::reset_to_defaults().map_err(|e| e)?;
        println!("[+] Reset complete.");
        return Ok(());
    }

    if let Some(ref prof_id) = cli.profile {
        println!("[*] Applying profile: {}...", prof_id);
        system::apply_profile_by_id(prof_id).map_err(|e| e)?;
        println!("[+] Successfully activated [{}] mode!", prof_id.to_uppercase());
        return Ok(());
    }

    let target_profile = if cli.home {
        Some(Profile::Home)
    } else if cli.work {
        Some(Profile::Work)
    } else if cli.dev {
        Some(Profile::Dev)
    } else if cli.travel || cli.secure {
        Some(Profile::Travel)
    } else {
        None
    };

    if let Some(profile) = target_profile {
        println!("[*] Applying profile: {}...", profile.display_name());
        system::apply_profile(profile).map_err(|e| e)?;
        println!("[+] Successfully activated [{}] mode!", profile.as_str().to_uppercase());
        return Ok(());
    }

    // Default: print status
    println!("{}", system::get_status_report());
    println!("\nUsage: postureflow-daemon [--home | --work | --dev | --travel | --score | --ports | --autoflow | --daemon | --status]");
    Ok(())
}

async fn run_daemon(session_bus: bool) -> Result<(), Box<dyn Error>> {
    println!("[*] Starting postureflow-daemon (v1.0.0)...");
    let service = PostureFlowService::new();

    let connection = if session_bus {
        println!("[*] Connecting to D-Bus Session Bus (Test mode)...");
        zbus::connection::Builder::session()?
            .name(DBUS_INTERFACE)?
            .serve_at(DBUS_PATH, service.clone())?
            .build()
            .await?
    } else {
        println!("[*] Connecting to D-Bus System Bus...");
        zbus::connection::Builder::system()?
            .name(DBUS_INTERFACE)?
            .serve_at(DBUS_PATH, service.clone())?
            .build()
            .await?
    };

    println!("[+] D-Bus Service registered at {}", DBUS_INTERFACE);
    // Shared atomic flag indicating if an app trigger is currently active
    let app_trigger_active = Arc::new(AtomicBool::new(false));

    // Spawn reactive Auto-Flow background watcher
    let service_clone = service.clone();
    let conn_clone = connection.clone();
    let app_trigger_flag_autoflow = app_trigger_active.clone();
    tokio::spawn(async move {
        println!("[+] Auto-Flow background monitor activated.");
        let mut last_applied_profile = String::new();
        let mut last_network_key = String::new();

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            let config = autoflow::load_autoflow_config();
            if !config.enabled {
                continue;
            }

            let net = autoflow::detect_active_networks();
            let network_key = format!(
                "{}:{}:{}",
                net.primary_type,
                net.current_ssid.as_deref().unwrap_or("none"),
                net.active_vpn.as_deref().unwrap_or("none")
            );

            if network_key != last_network_key {
                last_network_key = network_key;

                if !app_trigger_flag_autoflow.load(Ordering::SeqCst) {
                    if let Some(target_profile) = autoflow::evaluate_posture(&config, &net) {
                    let current = service_clone.get_active_profile_str().await;
                    if target_profile != current && target_profile != last_applied_profile {
                        println!(
                            "[*] Auto-Flow: Context shift (SSID: {:?}, VPN: {:?}) ➔ Auto-activating [{}]",
                            net.current_ssid,
                            net.active_vpn,
                            target_profile.to_uppercase()
                        );
                        if let Err(e) = system::apply_profile_by_id(&target_profile) {
                            eprintln!("[-] Auto-Flow: Failed to apply profile {}: {}", target_profile, e);
                        } else {
                            last_applied_profile = target_profile.clone();
                            service_clone.set_active_profile_str(&target_profile).await;

                            let _ = conn_clone.emit_signal(
                                Option::<&str>::None,
                                DBUS_PATH,
                                DBUS_INTERFACE,
                                "ProfileChanged",
                                &(&target_profile),
                            ).await;
                        }
                    }
                }
                }
            }
        }
    });



    // Spawn reactive App-Aware Dynamic Triggers background monitor
    let service_triggers = service.clone();
    let conn_triggers = connection.clone();
    let app_trigger_flag_triggers = app_trigger_active.clone();
    tokio::spawn(async move {
        println!("[+] App-Aware Dynamic Triggers background monitor activated.");
        let mut session = triggers::TriggerSession::default();
        let mut cached_cfg = triggers::load_triggers_config();
        let mut last_mtime = std::fs::metadata(triggers::triggers_config_path()).and_then(|m| m.modified()).ok();

        loop {
            // Check mtime to avoid unnecessary disk I/O on every tick
            let current_mtime = std::fs::metadata(triggers::triggers_config_path()).and_then(|m| m.modified()).ok();
            if current_mtime != last_mtime {
                cached_cfg = triggers::load_triggers_config();
                last_mtime = current_mtime;
            }

            let interval = cached_cfg.check_interval_seconds.max(1);
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;

            if !cached_cfg.enabled {
                if session.active_rule_name.is_some() {
                    revert_trigger_session(&mut session, &service_triggers, &conn_triggers, &app_trigger_flag_triggers).await;
                }
                continue;
            }

            let matched = triggers::evaluate_triggers_fast(&cached_cfg);

            match (session.active_rule_name.clone(), matched) {
                (None, Some(m)) => {
                    engage_trigger_session(&mut session, &m, &service_triggers, &conn_triggers, &app_trigger_flag_triggers).await;
                }
                (Some(active_name), Some(m)) => {
                    if active_name != m.rule_name {
                        revert_trigger_session(&mut session, &service_triggers, &conn_triggers, &app_trigger_flag_triggers).await;
                        engage_trigger_session(&mut session, &m, &service_triggers, &conn_triggers, &app_trigger_flag_triggers).await;
                    }
                }
                (Some(_), None) => {
                    revert_trigger_session(&mut session, &service_triggers, &conn_triggers, &app_trigger_flag_triggers).await;
                }
                (None, None) => {}
            }
        }
    });

    // Spawn reactive Circadian Schedule & Battery background monitor
    let service_sched = service.clone();
    let conn_sched = connection.clone();
    let app_trigger_flag_sched = app_trigger_active.clone();
    tokio::spawn(async move {
        println!("[+] Circadian Schedule & Battery background monitor activated.");
        let mut last_applied_profile = String::new();
        let mut was_emergency = false;
        let mut cached_cfg = schedule::load_schedule_config();
        let mut last_mtime = std::fs::metadata(schedule::schedule_config_path()).and_then(|m| m.modified()).ok();

        loop {
            // Check mtime to avoid unnecessary disk I/O on every tick
            let current_mtime = std::fs::metadata(schedule::schedule_config_path()).and_then(|m| m.modified()).ok();
            if current_mtime != last_mtime {
                cached_cfg = schedule::load_schedule_config();
                last_mtime = current_mtime;
            }

            let interval = cached_cfg.check_interval_seconds.max(5);
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;

            if !cached_cfg.enabled {
                continue;
            }

            let now = schedule::get_current_local_time();
            let battery = schedule::get_battery_status();
            let decision = schedule::evaluate_schedule(&cached_cfg, &now, &battery);

            match decision {
                Some(schedule::ScheduleDecision::BatteryEmergency {
                    ref rule_name,
                    ref target_profile,
                    battery_percent,
                    ref cpu_epp,
                    disable_bluetooth,
                }) => {
                    let current = service_sched.get_active_profile_str().await;
                    if !was_emergency || current != *target_profile {
                        println!(
                            "[!] Battery Critical ({}%): {} ➔ Activating Emergency Fallback [{}]",
                            battery_percent,
                            rule_name,
                            target_profile.to_uppercase()
                        );
                        if let Err(e) = system::apply_profile_by_id(target_profile) {
                            eprintln!("[-] Battery Emergency: Failed to apply profile {}: {}", target_profile, e);
                        } else {
                            if let Some(ref epp) = cpu_epp {
                                let _ = system::apply_cpu_epp(epp);
                            }
                            if disable_bluetooth {
                                let _ = system::apply_bluetooth(false);
                            }
                            was_emergency = true;
                            last_applied_profile = target_profile.clone();
                            service_sched.set_active_profile_str(target_profile).await;

                            let _ = conn_sched.emit_signal(
                                Option::<&str>::None,
                                DBUS_PATH,
                                DBUS_INTERFACE,
                                "ProfileChanged",
                                &(&target_profile),
                            ).await;
                        }
                    }
                }
                Some(schedule::ScheduleDecision::ScheduledShift {
                    ref rule_name,
                    ref target_profile,
                    ref window,
                }) => {
                    if was_emergency {
                        println!("[+] Battery Emergency cleared (AC connected or charged). Resuming schedule.");
                        was_emergency = false;
                    }

                    // Only apply scheduled shifts if an app trigger isn't actively boosting
                    if !app_trigger_flag_sched.load(Ordering::SeqCst) {
                        let current = service_sched.get_active_profile_str().await;
                        if target_profile != &current && target_profile != &last_applied_profile {
                            println!(
                                "[*] Circadian Schedule: '{}' ({}) ➔ Auto-activating [{}]",
                                rule_name,
                                window,
                                target_profile.to_uppercase()
                            );
                            if let Err(e) = system::apply_profile_by_id(target_profile) {
                                eprintln!("[-] Schedule: Failed to apply profile {}: {}", target_profile, e);
                            } else {
                                last_applied_profile = target_profile.clone();
                                service_sched.set_active_profile_str(target_profile).await;

                                let _ = conn_sched.emit_signal(
                                    Option::<&str>::None,
                                    DBUS_PATH,
                                    DBUS_INTERFACE,
                                    "ProfileChanged",
                                    &(&target_profile),
                                ).await;
                            }
                        }
                    }
                }
                None => {
                    if was_emergency {
                        println!("[+] Battery Emergency cleared. Resuming normal operations.");
                        was_emergency = false;
                    }
                }
            }
        }
    });

    tokio::signal::ctrl_c().await?;
    println!("\n[*] Shutting down postureflow-daemon...");
    drop(connection);
    Ok(())
}

async fn engage_trigger_session(
    session: &mut triggers::TriggerSession,
    m: &triggers::MatchedTrigger,
    service: &PostureFlowService,
    conn: &zbus::Connection,
    app_flag: &Arc<AtomicBool>,
) {
    let current_profile = service.get_active_profile_str().await;
    session.baseline_profile = Some(current_profile.clone());
    session.baseline_cpu_epp = system::get_cpu_epp();
    session.active_rule_name = Some(m.rule_name.clone());
    app_flag.store(true, Ordering::SeqCst);

    // Save baseline sysctls before applying boost
    for key in m.boost_sysctl.keys() {
        if let Some(val) = triggers::read_sysctl_value(key) {
            session.baseline_sysctls.insert(key.clone(), val);
        }
    }

    println!(
        "[*] App Trigger: '{}' activated (detected: {})",
        m.rule_name,
        m.matched_processes.join(", ")
    );

    // 1. Boost CPU EPP if requested
    if let Some(ref epp) = m.boost_cpu_epp {
        let _ = system::apply_cpu_epp(epp);
    }

    // 2. Boost sysctls
    for (k, v) in &m.boost_sysctl {
        let _ = triggers::apply_sysctl_override(k, v);
    }

    // 3. Switch profile if requested
    if let Some(ref target) = m.target_profile {
        if *target != current_profile {
            if let Err(e) = system::apply_profile_by_id(target) {
                eprintln!("[-] App Trigger: Failed to switch to profile {}: {}", target, e);
            } else {
                service.set_active_profile_str(target).await;
                let _ = conn.emit_signal(
                    Option::<&str>::None,
                    DBUS_PATH,
                    DBUS_INTERFACE,
                    "ProfileChanged",
                    &(&target),
                ).await;
            }
        }
    }
}

async fn revert_trigger_session(
    session: &mut triggers::TriggerSession,
    service: &PostureFlowService,
    conn: &zbus::Connection,
    app_flag: &Arc<AtomicBool>,
) {
    if let Some(ref name) = session.active_rule_name {
        println!("[*] App Trigger: '{}' processes exited. Restoring baseline posture...", name);
    }

    // 1. Revert sysctls
    for (k, v) in &session.baseline_sysctls {
        let _ = triggers::apply_sysctl_override(k, v);
    }
    session.baseline_sysctls.clear();

    // 2. Revert CPU EPP
    if let Some(ref epp) = session.baseline_cpu_epp {
        let _ = system::apply_cpu_epp(epp);
    }
    session.baseline_cpu_epp = None;

    // 3. Revert profile if baseline was different
    if let Some(ref base) = session.baseline_profile {
        let current = service.get_active_profile_str().await;
        if *base != current {
            if let Err(e) = system::apply_profile_by_id(base) {
                eprintln!("[-] App Trigger: Failed to revert to profile {}: {}", base, e);
            } else {
                service.set_active_profile_str(base).await;
                let _ = conn.emit_signal(
                    Option::<&str>::None,
                    DBUS_PATH,
                    DBUS_INTERFACE,
                    "ProfileChanged",
                    &(&base),
                ).await;
            }
        }
    }
    session.baseline_profile = None;
    session.active_rule_name = None;
    app_flag.store(false, Ordering::SeqCst);
    println!("[+] App Trigger: Baseline posture restored.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_profile_parsing() {
        assert_eq!(Profile::from_str("home").unwrap(), Profile::Home);
        assert_eq!(Profile::from_str("work").unwrap(), Profile::Work);
        assert_eq!(Profile::from_str("dev").unwrap(), Profile::Dev);
        assert_eq!(Profile::from_str("travel").unwrap(), Profile::Travel);
        assert_eq!(Profile::from_str("secure").unwrap(), Profile::Travel);
        assert!(Profile::from_str("invalid").is_err());
    }

    #[test]
    fn test_profile_display_and_icons() {
        assert_eq!(Profile::Home.as_str(), "home");
        assert_eq!(Profile::Home.icon_name(), "postureflow-home-symbolic");
        assert_eq!(Profile::Home.fallback_icon_name(), "user-home-symbolic");
        assert_eq!(Profile::Work.icon_name(), "postureflow-work-symbolic");
        assert_eq!(Profile::Work.fallback_icon_name(), "applications-office-symbolic");
        assert_eq!(Profile::Dev.icon_name(), "postureflow-dev-symbolic");
        assert_eq!(Profile::Dev.fallback_icon_name(), "utilities-terminal-symbolic");
        assert_eq!(Profile::Travel.as_str(), "travel");
        assert_eq!(Profile::Travel.icon_name(), "postureflow-travel-symbolic");
        assert_eq!(Profile::Travel.fallback_icon_name(), "security-high-symbolic");
    }
}
