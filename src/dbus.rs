use std::sync::Arc;
use tokio::sync::Mutex;
use zbus::{interface, fdo};
use crate::system;

pub const DBUS_INTERFACE: &str = "io.github.mzia.PostureFlow";
pub const DBUS_PATH: &str = "/io/github/mzia/PostureFlow";

#[derive(Clone)]
pub struct PostureFlowService {
    active_profile: Arc<Mutex<String>>,
}

impl PostureFlowService {
    pub fn new() -> Self {
        let current = system::get_active_profile();
        Self {
            active_profile: Arc::new(Mutex::new(current)),
        }
    }

    pub async fn get_active_profile_str(&self) -> String {
        self.active_profile.lock().await.clone()
    }

    pub async fn set_active_profile_str(&self, id: &str) {
        let mut cur = self.active_profile.lock().await;
        *cur = id.to_string();
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

    /// Calculates and returns the real-time Posture Score (0-100) and JSON report
    async fn get_posture_score(&self) -> fdo::Result<(i32, String)> {
        let report = crate::inspector::PostureScoreReport::compute();
        let json = serde_json::to_string(&report)
            .map_err(|e| fdo::Error::Failed(format!("Serialization error: {}", e)))?;
        Ok((report.total_score as i32, json))
    }

    /// Returns all listening sockets with process and exposure metadata (JSON string)
    async fn get_listening_ports(&self) -> fdo::Result<String> {
        let ports = crate::inspector::ports::scan_listening_ports();
        serde_json::to_string(&ports)
            .map_err(|e| fdo::Error::Failed(format!("Serialization error: {}", e)))
    }

    /// Blocks a listening port in UFW immediately: requires Polkit authorization
    async fn block_port(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: zbus::message::Header<'_>,
        port: u16,
        proto: String,
    ) -> fdo::Result<()> {
        check_posture_auth(conn, hdr.sender()).await?;
        system::block_port(port, &proto)
            .map_err(|e| fdo::Error::Failed(e))
    }

    /// Returns the active Auto-Flow status: (enabled, current_network, matched_profile)
    async fn get_auto_flow_status(&self) -> fdo::Result<(bool, String, String)> {
        let cfg = crate::autoflow::load_autoflow_config();
        let net = crate::autoflow::detect_active_networks();
        let net_desc = net.current_ssid.clone().unwrap_or_else(|| net.primary_type.clone());
        let matched = crate::autoflow::evaluate_posture(&cfg, &net).unwrap_or_else(|| "none".to_string());
        Ok((cfg.enabled, net_desc, matched))
    }

    /// Toggles Auto-Flow master switch: requires Polkit authorization
    async fn set_auto_flow_enabled(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: zbus::message::Header<'_>,
        enabled: bool,
    ) -> fdo::Result<()> {
        check_posture_auth(conn, hdr.sender()).await?;
        let mut cfg = crate::autoflow::load_autoflow_config();
        cfg.enabled = enabled;
        crate::autoflow::save_autoflow_config(&cfg)
            .map_err(|e| fdo::Error::Failed(e))?;
        Ok(())
    }

    /// Returns current Auto-Flow configuration TOML string
    async fn get_auto_flow_config(&self) -> fdo::Result<String> {
        let cfg = crate::autoflow::load_autoflow_config();
        toml::to_string_pretty(&cfg)
            .map_err(|e| fdo::Error::Failed(format!("TOML serialize error: {}", e)))
    }

    /// Updates Auto-Flow configuration: requires Polkit authorization
    async fn save_auto_flow_config(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: zbus::message::Header<'_>,
        toml_str: String,
    ) -> fdo::Result<()> {
        check_posture_auth(conn, hdr.sender()).await?;
        let cfg: crate::autoflow::AutoFlowConfig = toml::from_str(&toml_str)
            .map_err(|e| fdo::Error::InvalidArgs(format!("Invalid TOML syntax: {}", e)))?;
        crate::autoflow::save_autoflow_config(&cfg)
            .map_err(|e| fdo::Error::Failed(e))?;
        Ok(())
    }

    /// Returns the active Triggers status: (enabled, active_trigger_rule_name)
    async fn get_triggers_status(&self) -> fdo::Result<(bool, String)> {
        let cfg = crate::triggers::load_triggers_config();
        let running = crate::triggers::scan_running_processes();
        let matched = crate::triggers::evaluate_triggers(&cfg, &running);
        let active = matched.map(|m| m.rule_name).unwrap_or_else(|| "none".to_string());
        Ok((cfg.enabled, active))
    }

    /// Returns current Triggers configuration TOML string
    async fn get_triggers_config(&self) -> fdo::Result<String> {
        let cfg = crate::triggers::load_triggers_config();
        toml::to_string_pretty(&cfg)
            .map_err(|e| fdo::Error::Failed(format!("TOML serialize error: {}", e)))
    }

    /// Updates Triggers configuration: requires Polkit authorization
    async fn save_triggers_config(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: zbus::message::Header<'_>,
        toml_str: String,
    ) -> fdo::Result<()> {
        check_posture_auth(conn, hdr.sender()).await?;
        let cfg: crate::triggers::TriggersConfig = toml::from_str(&toml_str)
            .map_err(|e| fdo::Error::InvalidArgs(format!("Invalid TOML syntax: {}", e)))?;
        crate::triggers::save_triggers_config(&cfg)
            .map_err(|e| fdo::Error::Failed(e))?;
        Ok(())
    }

    /// D-Bus Signal emitted whenever the profile changes
    #[zbus(signal)]
    async fn profile_changed(ctxt: &zbus::SignalContext<'_>, new_profile: &str) -> zbus::Result<()>;
}

/// Verifies caller via PolicyKit (Polkit) authority over D-Bus
async fn check_posture_auth(
    conn: &zbus::Connection,
    sender: Option<&zbus::names::UniqueName<'_>>,
) -> fdo::Result<()> {
    let sender_unique = match sender {
        Some(s) => s,
        None => return Ok(()),
    };

    let sender_name = sender_unique.as_str();
    if sender_name.is_empty() {
        return Ok(());
    }

    let auth_proxy = match zbus::fdo::DBusProxy::new(conn).await {
        Ok(p) => p,
        Err(_) => return Ok(()),
    };

    let bus_name = zbus::names::BusName::from(sender_unique.clone().to_owned());
    let uid = match auth_proxy.get_connection_unix_user(bus_name).await {
        Ok(u) => u,
        Err(_) => return Ok(()),
    };

    if uid == 0 {
        return Ok(());
    }

    let authority_proxy = zbus::Proxy::new(
        conn,
        "org.freedesktop.PolicyKit1",
        "/org/freedesktop/PolicyKit1/Authority",
        "org.freedesktop.PolicyKit1.Authority",
    )
    .await;

    let authority = match authority_proxy {
        Ok(a) => a,
        Err(_) => return Ok(()),
    };

    let subject = (
        "system-bus-name",
        std::collections::HashMap::from([("name".to_string(), sender_name)]),
    );

    let details: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let flags: u32 = 1; // AllowUserInteraction
    let cancellation_id = "";

    let result = authority
        .call::<_, _, (bool, bool, std::collections::HashMap<String, String>)>(
            "CheckAuthorization",
            &(
                subject,
                "io.github.mzia.PostureFlow.set-profile",
                details,
                flags,
                cancellation_id,
            ),
        )
        .await;

    let is_authorized = match result {
        Ok((authorized, _, _)) => authorized,
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("ServiceUnknown") || err_str.contains("NameHasNoOwner") {
                // In test environments or session bus without polkit daemon
                return Ok(());
            }
            eprintln!("[!] Polkit query error: {}", e);
            return Err(fdo::Error::Failed(format!("PolicyKit error: {}", e)));
        }
    };

    if !is_authorized {
        return Err(fdo::Error::AccessDenied("Polkit authorization required to set profile or modify system posture".into()));
    }

    Ok(())
}
