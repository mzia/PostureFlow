use std::error::Error;
use clap::Parser;
use postureflow::profile::Profile;
use postureflow::dbus::{
    PostureFlowService, LegacyPopProfileService,
    DBUS_INTERFACE, DBUS_PATH, LEGACY_DBUS_INTERFACE, LEGACY_DBUS_PATH,
};
use postureflow::system;

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
    println!("\nUsage: postureflow-daemon [--home | --work | --dev | --travel | --daemon | --status]");
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
            .serve_at(DBUS_PATH, service)?
            .serve_at(LEGACY_DBUS_PATH, legacy_service)?
            .build()
            .await?
    } else {
        println!("[*] Connecting to D-Bus System Bus...");
        zbus::connection::Builder::system()?
            .name(DBUS_INTERFACE)?
            .name(LEGACY_DBUS_INTERFACE)?
            .serve_at(DBUS_PATH, service)?
            .serve_at(LEGACY_DBUS_PATH, legacy_service)?
            .build()
            .await?
    };

    println!("[+] D-Bus Service registered at {} and {} (compat)", DBUS_INTERFACE, LEGACY_DBUS_INTERFACE);
    println!("[+] Daemon ready and listening for COSMIC Applet requests. Press Ctrl+C to stop.");

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
