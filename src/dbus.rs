use std::sync::Arc;
use tokio::sync::Mutex;
use zbus::{interface, fdo};
use crate::system;

pub const DBUS_INTERFACE: &str = "io.github.mzia.PostureFlow";
pub const DBUS_PATH: &str = "/io/github/mzia/PostureFlow";
pub const LEGACY_DBUS_INTERFACE: &str = "io.github.mzia.PopProfile";
pub const LEGACY_DBUS_PATH: &str = "/io/github/mzia/PopProfile";

#[derive(Clone)]
pub struct PostureFlowService {
    active_profile: Arc<Mutex<String>>,
}

pub type PopProfileService = PostureFlowService;

impl PostureFlowService {
    pub fn new() -> Self {
        let current = system::get_active_profile();
        Self {
            active_profile: Arc::new(Mutex::new(current)),
        }
    }
}

#[interface(name = "io.github.mzia.PostureFlow")]
impl PostureFlowService {
    /// Returns the active profile name ("home", "work", "dev", "travel", or "default")
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
        check_posture_auth(conn, hdr.sender()).await?;

        let target_id = profile.trim().to_lowercase();
        system::apply_profile_by_id(&target_id)
            .map_err(|e| fdo::Error::Failed(format!("Failed to apply profile: {}", e)))?;

        let mut current = self.active_profile.lock().await;
        *current = target_id.clone();

        // Emit ProfileChanged signal
        Self::profile_changed(&ctxt, &target_id)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        Ok(())
    }

    /// Returns list of available profiles: (id, name, description, icon, is_active)
    async fn list_profiles(&self) -> Vec<(String, String, String, String, bool)> {
        let active = self.active_profile.lock().await.clone();
        crate::config::load_all_profiles()
            .into_iter()
            .map(|p| {
                let is_active = p.profile.id == active
                    || (active == "default" && p.profile.id == "home");
                (
                    p.profile.id,
                    p.profile.name,
                    p.profile.description,
                    p.profile.icon,
                    is_active,
                )
            })
            .collect()
    }

    /// Returns full TOML configuration for a specific profile ID
    async fn get_profile_details(&self, id: String) -> fdo::Result<String> {
        let conf = crate::config::find_profile(&id)
            .ok_or_else(|| fdo::Error::InvalidArgs(format!("Profile '{}' not found", id)))?;
        conf.to_toml()
            .map_err(|e| fdo::Error::Failed(format!("Failed to serialize TOML: {}", e)))
    }

    /// Validates a custom profile TOML without applying: returns (is_valid, warnings, sanitized_toml)
    async fn validate_profile(&self, toml_content: String) -> fdo::Result<(bool, Vec<String>, String)> {
        let parsed = crate::config::ProfileConfig::from_toml(&toml_content)
            .map_err(|e| fdo::Error::InvalidArgs(format!("TOML syntax error: {}", e)))?;
        let report = crate::config::validate_and_sanitize(parsed)
            .map_err(|e| fdo::Error::InvalidArgs(format!("Safety validation failed: {}", e)))?;
        let sanitized_toml = report
            .sanitized_config
            .to_toml()
            .map_err(|e| fdo::Error::Failed(format!("TOML serialization error: {}", e)))?;
        Ok((report.is_valid, report.warnings, sanitized_toml))
    }

    /// Saves a custom profile: requires Polkit authorization
    async fn save_custom_profile(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: zbus::message::Header<'_>,
        toml_content: String,
    ) -> fdo::Result<String> {
        check_posture_auth(conn, hdr.sender()).await?;

        let parsed = crate::config::ProfileConfig::from_toml(&toml_content)
            .map_err(|e| fdo::Error::InvalidArgs(format!("TOML syntax error: {}", e)))?;
        let is_system = system::is_privileged();
        let path = crate::config::save_custom_profile(parsed, is_system)
            .map_err(|e| fdo::Error::Failed(e))?;
        Ok(path.to_string_lossy().to_string())
    }

    /// Deletes a custom profile: requires Polkit authorization
    async fn delete_custom_profile(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: zbus::message::Header<'_>,
        id: String,
    ) -> fdo::Result<()> {
        check_posture_auth(conn, hdr.sender()).await?;

        crate::config::delete_custom_profile(&id)
            .map_err(|e| fdo::Error::Failed(e))
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
        check_posture_auth(conn, hdr.sender()).await?;

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

/// Backwards-compatibility wrapper serving io.github.mzia.PopProfile
#[derive(Clone)]
pub struct LegacyPopProfileService(pub PostureFlowService);

#[interface(name = "io.github.mzia.PopProfile")]
impl LegacyPopProfileService {
    async fn get_active_profile(&self) -> String {
        self.0.get_active_profile().await
    }

    async fn set_profile(
        &self,
        #[zbus(signal_context)] ctxt: zbus::SignalContext<'_>,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: zbus::message::Header<'_>,
        profile: String,
    ) -> fdo::Result<()> {
        self.0.set_profile(ctxt, conn, hdr, profile).await
    }

    async fn list_profiles(&self) -> Vec<(String, String, String, String, bool)> {
        self.0.list_profiles().await
    }

    async fn get_profile_details(&self, id: String) -> fdo::Result<String> {
        self.0.get_profile_details(id).await
    }

    async fn validate_profile(&self, toml_content: String) -> fdo::Result<(bool, Vec<String>, String)> {
        self.0.validate_profile(toml_content).await
    }

    async fn save_custom_profile(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: zbus::message::Header<'_>,
        toml_content: String,
    ) -> fdo::Result<String> {
        self.0.save_custom_profile(conn, hdr, toml_content).await
    }

    async fn delete_custom_profile(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: zbus::message::Header<'_>,
        id: String,
    ) -> fdo::Result<()> {
        self.0.delete_custom_profile(conn, hdr, id).await
    }

    async fn get_status(&self) -> String {
        self.0.get_status().await
    }

    async fn reset_to_defaults(
        &self,
        #[zbus(signal_context)] ctxt: zbus::SignalContext<'_>,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: zbus::message::Header<'_>,
    ) -> fdo::Result<()> {
        self.0.reset_to_defaults(ctxt, conn, hdr).await
    }

    #[zbus(signal)]
    async fn profile_changed(ctxt: &zbus::SignalContext<'_>, new_profile: &str) -> zbus::Result<()>;
}

async fn check_posture_auth(
    conn: &zbus::Connection,
    sender: Option<&zbus::names::UniqueName<'_>>,
) -> Result<(), fdo::Error> {
    if check_polkit_auth(conn, sender, "io.github.mzia.PostureFlow.set-profile").await.is_err() {
        check_polkit_auth(conn, sender, "io.github.mzia.PopProfile.set-profile").await?;
    }
    Ok(())
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
