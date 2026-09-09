use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use zbus::{interface, fdo};
use crate::profile::Profile;
use crate::system;

pub const DBUS_INTERFACE: &str = "io.github.mzia.PopProfile";
pub const DBUS_PATH: &str = "/io/github/mzia/PopProfile";

#[derive(Clone)]
pub struct PopProfileService {
    active_profile: Arc<Mutex<String>>,
}

impl PopProfileService {
    pub fn new() -> Self {
        let current = system::get_active_profile();
        Self {
            active_profile: Arc::new(Mutex::new(current)),
        }
    }
}

#[interface(name = "io.github.mzia.PopProfile")]
impl PopProfileService {
    /// Returns the active profile name ("home", "work", "dev", "secure", or "default")
    async fn get_active_profile(&self) -> String {
        let profile = self.active_profile.lock().await;
        profile.clone()
    }

    /// Sets the active profile and applies system changes
    async fn set_profile(
        &self,
        #[zbus(signal_context)] ctxt: zbus::SignalContext<'_>,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: zbus::message::Header<'_>,
        profile: String,
    ) -> fdo::Result<()> {
        check_polkit_auth(conn, hdr.sender(), "io.github.mzia.PopProfile.set-profile").await?;

        let parsed = Profile::from_str(&profile)
            .map_err(|e| fdo::Error::InvalidArgs(e))?;

        system::apply_profile(parsed)
            .map_err(|e| fdo::Error::Failed(format!("Failed to apply profile: {}", e)))?;

        let mut current = self.active_profile.lock().await;
        *current = parsed.as_str().to_string();

        // Emit ProfileChanged signal
        Self::profile_changed(&ctxt, parsed.as_str())
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        Ok(())
    }

    /// Returns the live system status report
    async fn get_status(&self) -> String {
        system::get_status_report()
    }

    /// Resets firewall and sysctl to factory Pop!_OS defaults
    async fn reset_to_defaults(
        &self,
        #[zbus(signal_context)] ctxt: zbus::SignalContext<'_>,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: zbus::message::Header<'_>,
    ) -> fdo::Result<()> {
        check_polkit_auth(conn, hdr.sender(), "io.github.mzia.PopProfile.set-profile").await?;

        system::reset_to_defaults()
            .map_err(|e| fdo::Error::Failed(e))?;

        let mut current = self.active_profile.lock().await;
        *current = "default".to_string();

        let _ = Self::profile_changed(&ctxt, "default").await;
        Ok(())
    }

    /// D-Bus Signal emitted whenever the profile changes
    #[zbus(signal)]
    async fn profile_changed(ctxt: &zbus::SignalContext<'_>, new_profile: &str) -> zbus::Result<()>;
}

async fn check_polkit_auth(
    conn: &zbus::Connection,
    sender: Option<&zbus::names::UniqueName<'_>>,
    action_id: &str,
) -> Result<(), fdo::Error> {
    if !system::is_privileged() {
        return Ok(());
    }

    let Some(sender_name) = sender else {
        return Err(fdo::Error::AccessDenied("Sender bus name missing".into()));
    };

    let mut subject_details: std::collections::HashMap<&str, zbus::zvariant::Value<'_>> =
        std::collections::HashMap::new();
    subject_details.insert("name", zbus::zvariant::Value::from(sender_name.as_str()));
    let subject = ("system-bus-name", subject_details);

    let details: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    let flags: u32 = 1; // 1 = AllowUserInteraction (triggers GUI password dialog if needed)
    let cancellation_id = "";

    let authority_proxy = match zbus::Proxy::new(
        conn,
        "org.freedesktop.PolicyKit1",
        "/org/freedesktop/PolicyKit1/Authority",
        "org.freedesktop.PolicyKit1.Authority",
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[!] Warning: Could not connect to Polkit: {}", e);
            return Ok(());
        }
    };

    let result: Result<(bool, bool, std::collections::HashMap<String, String>), zbus::Error> =
        authority_proxy
            .call(
                "CheckAuthorization",
                &(subject, action_id, details, flags, cancellation_id),
            )
            .await;

    match result {
        Ok((is_authorized, _is_challenge, _)) => {
            if is_authorized {
                Ok(())
            } else {
                Err(fdo::Error::AccessDenied(
                    "Polkit authentication rejected or cancelled".into(),
                ))
            }
        }
        Err(e) => {
            eprintln!("[!] Polkit CheckAuthorization call failed: {}", e);
            Err(fdo::Error::Failed(format!("Polkit error: {}", e)))
        }
    }
}
