use std::error::Error;
use std::collections::HashMap;
use tokio::sync::mpsc;
use zbus::Connection;
use zbus::zvariant::Value;
use crate::profile::Profile;
use crate::dbus::{DBUS_INTERFACE, DBUS_PATH};

#[derive(Clone)]
pub struct DaemonClient {
    active_conn: Connection,
    session_conn: Connection,
}

impl DaemonClient {
    pub async fn connect(force_session_bus: bool) -> Result<Self, Box<dyn Error>> {
        let session_conn = Connection::session().await?;
        let active_conn = if force_session_bus {
            println!("[*] Connecting to D-Bus Session Bus (test mode)...");
            session_conn.clone()
        } else {
            match Connection::system().await {
                Ok(sys) => {
                    println!("[*] Connected to D-Bus System Bus.");
                    sys
                }
                Err(e) => {
                    eprintln!("[!] System bus unavailable ({}); falling back to Session Bus...", e);
                    session_conn.clone()
                }
            }
        };

        Ok(Self {
            active_conn,
            session_conn,
        })
    }

    pub fn session_conn(&self) -> &Connection {
        &self.session_conn
    }

    pub async fn get_active_profile(&self) -> Result<Profile, Box<dyn Error>> {
        let proxy = zbus::Proxy::new(
            &self.active_conn,
            DBUS_INTERFACE,
            DBUS_PATH,
            DBUS_INTERFACE,
        ).await?;
        let res: String = proxy.call("GetActiveProfile", &()).await?;
        res.parse().map_err(|e: String| e.into())
    }

    pub async fn set_profile(&self, profile: Profile) -> Result<(), Box<dyn Error>> {
        let proxy = zbus::Proxy::new(
            &self.active_conn,
            DBUS_INTERFACE,
            DBUS_PATH,
            DBUS_INTERFACE,
        ).await?;
        let _: () = proxy.call("SetProfile", &(profile.as_str(),)).await?;
        Ok(())
    }

    pub async fn get_status(&self) -> Result<String, Box<dyn Error>> {
        let proxy = zbus::Proxy::new(
            &self.active_conn,
            DBUS_INTERFACE,
            DBUS_PATH,
            DBUS_INTERFACE,
        ).await?;
        let res: String = proxy.call("GetStatus", &()).await?;
        Ok(res)
    }

    pub async fn reset_to_defaults(&self) -> Result<(), Box<dyn Error>> {
        let proxy = zbus::Proxy::new(
            &self.active_conn,
            DBUS_INTERFACE,
            DBUS_PATH,
            DBUS_INTERFACE,
        ).await?;
        let _: () = proxy.call("ResetToDefaults", &()).await?;
        Ok(())
    }

    pub async fn listen_profile_changed(&self, tx: mpsc::Sender<Profile>) -> Result<(), Box<dyn Error>> {
        let proxy = zbus::Proxy::new(
            &self.active_conn,
            DBUS_INTERFACE,
            DBUS_PATH,
            DBUS_INTERFACE,
        ).await?;
        let mut stream = proxy.receive_signal("ProfileChanged").await?;

        tokio::spawn(async move {
            use futures_util::StreamExt;
            while let Some(msg) = stream.next().await {
                if let Ok((profile_str,)) = msg.body().deserialize::<(String,)>() {
                    if let Ok(p) = profile_str.parse::<Profile>() {
                        let _ = tx.send(p).await;
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn send_notification(&self, summary: &str, body: &str, icon: &str) {
        if let Ok(proxy) = zbus::Proxy::new(
            &self.session_conn,
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
        ).await {
            let actions: Vec<String> = Vec::new();
            let hints = HashMap::<String, Value>::new();
            let _: Result<u32, _> = proxy.call(
                "Notify",
                &(
                    "PostureFlow",
                    0u32,
                    icon,
                    summary,
                    body,
                    actions,
                    hints,
                    5000i32,
                ),
            ).await;
        }
    }
}
