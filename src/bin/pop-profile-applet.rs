use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use clap::Parser;
use futures_util::StreamExt;
use tokio::sync::{mpsc, RwLock};
use zbus::Connection;
use zbus::fdo::DBusProxy;

use pop_profile::applet::{
    AppState, DaemonClient, DbusMenu, MenuAction, StatusNotifierItem,
    MENU_OBJECT_PATH, SNI_OBJECT_PATH, WATCHER_BUS_NAME, WATCHER_OBJECT_PATH,
};
use pop_profile::profile::Profile;

#[derive(Parser, Debug)]
#[command(name = "pop-profile-applet")]
#[command(author = "M. Zia")]
#[command(version = "1.0.0")]
#[command(about = "COSMIC Panel Applet & System Tray for Pop! Profile Manager", long_about = None)]
struct Cli {
    /// Connect to Session Bus instead of System Bus for daemon communication (test mode)
    #[arg(long)]
    session_bus: bool,

    /// Query and print current daemon status, then exit
    #[arg(long, short = 's')]
    status: bool,

    /// Cycle to the next profile and exit
    #[arg(long, short = 'c')]
    cycle: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    println!("[*] Initializing Pop! Profile Applet (v1.0.0)...");
    let daemon = DaemonClient::connect(cli.session_bus).await?;

    if cli.status {
        match daemon.get_status().await {
            Ok(status) => println!("{}", status),
            Err(e) => eprintln!("[-] Failed to query status: {}", e),
        }
        return Ok(());
    }

    if cli.cycle {
        let active = daemon.get_active_profile().await.unwrap_or(Profile::Home);
        let next = active.next();
        println!("[*] Cycling from {} to {}...", active.as_str(), next.as_str());
        daemon.set_profile(next).await?;
        println!("[+] Switched to {}", next.as_str().to_uppercase());
        return Ok(());
    }

    // Determine initial profile
    let initial_profile = match daemon.get_active_profile().await {
        Ok(p) => {
            println!("[+] Connected to daemon. Active profile: [{}]", p.as_str().to_uppercase());
            p
        }
        Err(e) => {
            eprintln!("[!] Could not fetch active profile from daemon ({}). Defaulting to Home.", e);
            Profile::Home
        }
    };

    let state = Arc::new(RwLock::new(AppState::new(initial_profile)));
    let (action_tx, mut action_rx) = mpsc::channel::<MenuAction>(32);

    let session_conn = daemon.session_conn().clone();

    // Register StatusNotifierItem and DBusMenu
    let sni = StatusNotifierItem::new(state.clone(), action_tx.clone());
    let menu = DbusMenu::new(state.clone(), action_tx.clone());

    session_conn.object_server().at(SNI_OBJECT_PATH, sni).await?;
    session_conn.object_server().at(MENU_OBJECT_PATH, menu).await?;

    println!("[+] Registered D-Bus interfaces at {} and {}", SNI_OBJECT_PATH, MENU_OBJECT_PATH);

    // Register with StatusNotifierWatcher
    register_with_watcher(&session_conn).await;

    // Spawn watcher watchdog: re-registers if cosmic-panel or watcher restarts
    let session_conn_clone = session_conn.clone();
    tokio::spawn(async move {
        if let Ok(dbus_proxy) = DBusProxy::new(&session_conn_clone).await {
            if let Ok(mut stream) = dbus_proxy.receive_name_owner_changed().await {
                while let Some(signal) = stream.next().await {
                    if let Ok(args) = signal.args() {
                        if args.name() == WATCHER_BUS_NAME && args.new_owner().is_some() {
                            println!("[*] StatusNotifierWatcher restarted. Re-registering applet...");
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            register_with_watcher(&session_conn_clone).await;
                        }
                    }
                }
            }
        }
    });

    // Spawn listener for ProfileChanged signals from the system daemon
    let (signal_tx, mut signal_rx) = mpsc::channel::<Profile>(16);
    if let Err(e) = daemon.listen_profile_changed(signal_tx).await {
        eprintln!("[!] Warning: Could not subscribe to ProfileChanged signal: {}", e);
    }

    println!("[+] Applet successfully loaded into COSMIC Panel. Listening for interactions...");

    // Main event loop
    loop {
        tokio::select! {
            // Signal from system daemon: external profile change (e.g. from CLI or script)
            Some(new_profile) = signal_rx.recv() => {
                let changed = {
                    let mut st = state.write().await;
                    st.set_profile(new_profile)
                };
                if changed {
                    println!("[*] Daemon signal: Active profile changed to [{}]", new_profile.as_str().to_uppercase());
                    emit_profile_updated(&session_conn, &state).await;
                    daemon.send_notification(
                        "Pop! Profile Manager",
                        &format!("Profile switched to [{}]", new_profile.as_str().to_uppercase()),
                        new_profile.icon_name(),
                    ).await;
                }
            }

            // Action from user clicking menu or panel icon
            Some(action) = action_rx.recv() => {
                match action {
                    MenuAction::SwitchProfile(target) => {
                        let current = state.read().await.active_profile;
                        if current == target {
                            println!("[*] Profile already [{}]", target.as_str().to_uppercase());
                            continue;
                        }

                        println!("[*] Switching profile to [{}]...", target.as_str().to_uppercase());
                        daemon.send_notification(
                            "Pop! Profile Manager",
                            &format!("Applying profile: {}...", target.display_name()),
                            target.icon_name(),
                        ).await;

                        match daemon.set_profile(target).await {
                            Ok(()) => {
                                {
                                    let mut st = state.write().await;
                                    st.set_profile(target);
                                }
                                emit_profile_updated(&session_conn, &state).await;
                                daemon.send_notification(
                                    "Pop! Profile Manager",
                                    &format!("Active profile is now [{}]", target.as_str().to_uppercase()),
                                    target.icon_name(),
                                ).await;
                                println!("[+] Successfully activated [{}] mode!", target.as_str().to_uppercase());
                            }
                            Err(e) => {
                                eprintln!("[-] Failed to switch profile: {}", e);
                                daemon.send_notification(
                                    "Pop! Profile Error",
                                    &format!("Failed to switch profile: {}", e),
                                    "dialog-error-symbolic",
                                ).await;
                            }
                        }
                    }

                    MenuAction::ShowStatus => {
                        println!("[*] Fetching system status from daemon...");
                        match daemon.get_status().await {
                            Ok(status) => {
                                let lines: Vec<&str> = status.lines().take(6).collect();
                                let summary = lines.join("\n");
                                let active = state.read().await.active_profile;
                                daemon.send_notification(
                                    "Pop! Profile & Power Posture",
                                    &summary,
                                    active.icon_name(),
                                ).await;
                            }
                            Err(e) => {
                                eprintln!("[-] Failed to query status: {}", e);
                            }
                        }
                    }

                    MenuAction::ResetDefaults => {
                        println!("[*] Resetting to Pop!_OS factory defaults...");
                        match daemon.reset_to_defaults().await {
                            Ok(()) => {
                                {
                                    let mut st = state.write().await;
                                    st.set_profile(Profile::Home);
                                }
                                emit_profile_updated(&session_conn, &state).await;
                                daemon.send_notification(
                                    "Pop! Profile Manager",
                                    "Firewall and system power settings restored to factory defaults.",
                                    "security-medium-symbolic",
                                ).await;
                                println!("[+] Reset to defaults complete.");
                            }
                            Err(e) => {
                                eprintln!("[-] Failed to reset defaults: {}", e);
                            }
                        }
                    }

                    MenuAction::Quit => {
                        println!("[*] Quit action received. Exiting applet...");
                        break;
                    }
                }
            }

            _ = tokio::signal::ctrl_c() => {
                println!("\n[*] Shutting down Pop! Profile Applet...");
                break;
            }
        }
    }

    Ok(())
}

async fn register_with_watcher(conn: &Connection) {
    match zbus::Proxy::new(
        conn,
        WATCHER_BUS_NAME,
        WATCHER_OBJECT_PATH,
        WATCHER_BUS_NAME,
    ).await {
        Ok(proxy) => {
            let res: Result<(), _> = proxy.call("RegisterStatusNotifierItem", &(SNI_OBJECT_PATH,)).await;
            match res {
                Ok(()) => println!("[+] Successfully registered with COSMIC StatusNotifierWatcher!"),
                Err(e) => eprintln!("[!] Watcher registration notice: {}", e),
            }
        }
        Err(e) => {
            eprintln!("[!] Could not connect to StatusNotifierWatcher: {}", e);
        }
    }
}

async fn emit_profile_updated(conn: &Connection, state: &Arc<RwLock<AppState>>) {
    let rev = state.read().await.menu_revision;

    if let Ok(sni_ctxt) = zbus::SignalContext::new(conn, SNI_OBJECT_PATH) {
        let _ = StatusNotifierItem::new_icon(&sni_ctxt).await;
        let _ = StatusNotifierItem::new_tool_tip(&sni_ctxt).await;
        let _ = StatusNotifierItem::new_title(&sni_ctxt).await;
    }

    if let Ok(menu_ctxt) = zbus::SignalContext::new(conn, MENU_OBJECT_PATH) {
        let _ = DbusMenu::layout_updated(&menu_ctxt, rev, 0).await;
    }
}
