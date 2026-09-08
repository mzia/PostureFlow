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
    async fn set_profile(&self, #[zbus(signal_context)] ctxt: zbus::SignalContext<'_>, profile: String) -> fdo::Result<()> {
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
    async fn reset_to_defaults(&self, #[zbus(signal_context)] ctxt: zbus::SignalContext<'_>) -> fdo::Result<()> {
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
