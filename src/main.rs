use std::error::Error;
use clap::Parser;
use postureflow::profile::Profile;
use postureflow::dbus::{
    PostureFlowService, LegacyPopProfileService,
    DBUS_INTERFACE, DBUS_PATH, LEGACY_DBUS_INTERFACE, LEGACY_DBUS_PATH,
};
use postureflow::system;
use postureflow::inspector::{ports, PostureScoreReport};
use postureflow::autoflow;

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
    let legacy_service = LegacyPopProfileService(service.clone());

    let connection = if session_bus {
        println!("[*] Connecting to D-Bus Session Bus (Test mode)...");
        zbus::connection::Builder::session()?
            .name(DBUS_INTERFACE)?
            .name(LEGACY_DBUS_INTERFACE)?
            .serve_at(DBUS_PATH, service.clone())?
            .serve_at(LEGACY_DBUS_PATH, legacy_service)?
            .build()
            .await?
    } else {
        println!("[*] Connecting to D-Bus System Bus...");
        zbus::connection::Builder::system()?
            .name(DBUS_INTERFACE)?
            .name(LEGACY_DBUS_INTERFACE)?
            .serve_at(DBUS_PATH, service.clone())?
            .serve_at(LEGACY_DBUS_PATH, legacy_service)?
            .build()
            .await?
    };

    println!("[+] D-Bus Service registered at {} and {} (compat)", DBUS_INTERFACE, LEGACY_DBUS_INTERFACE);
    println!("[+] Daemon ready and listening for requests.");

    // Spawn reactive Auto-Flow background watcher
    let service_clone = service.clone();
    let conn_clone = connection.clone();
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
                            let _ = conn_clone.emit_signal(
                                Option::<&str>::None,
                                LEGACY_DBUS_PATH,
                                LEGACY_DBUS_INTERFACE,
                                "ProfileChanged",
                                &(&target_profile),
                            ).await;
                        }
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
        assert_eq!(Profile::Home.icon_name(), "user-home-symbolic");
        assert_eq!(Profile::Work.icon_name(), "applications-office-symbolic");
        assert_eq!(Profile::Dev.icon_name(), "utilities-terminal-symbolic");
        assert_eq!(Profile::Travel.as_str(), "travel");
        assert_eq!(Profile::Travel.icon_name(), "security-high-symbolic");
    }
}
