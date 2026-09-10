use tokio::sync::mpsc;
use zbus::interface;
use zbus::zvariant::OwnedObjectPath;
use crate::applet::state::SharedState;
use crate::applet::menu::MenuAction;

pub struct StatusNotifierItem {
    state: SharedState,
    action_tx: mpsc::Sender<MenuAction>,
}

impl StatusNotifierItem {
    pub fn new(state: SharedState, action_tx: mpsc::Sender<MenuAction>) -> Self {
        Self { state, action_tx }
    }
}

#[interface(name = "org.kde.StatusNotifierItem")]
impl StatusNotifierItem {
    #[zbus(property)]
    async fn category(&self) -> &str {
        "SystemServices"
    }

    #[zbus(property)]
    async fn id(&self) -> &str {
        "postureflow"
    }

    #[zbus(property)]
    async fn title(&self) -> String {
        let state = self.state.read().await;
        format!("PostureFlow ({})", state.active_profile.as_str().to_uppercase())
    }

    #[zbus(property)]
    async fn status(&self) -> &str {
        "Active"
    }

    #[zbus(property)]
    async fn window_id(&self) -> i32 {
        0
    }

    #[zbus(property)]
    async fn icon_theme_path(&self) -> &str {
        ""
    }

    #[zbus(property)]
    async fn menu(&self) -> OwnedObjectPath {
        OwnedObjectPath::try_from("/com/canonical/dbusmenu").unwrap()
    }

    #[zbus(property)]
    async fn item_is_menu(&self) -> bool {
        false
    }

    #[zbus(property)]
    async fn icon_name(&self) -> String {
        let state = self.state.read().await;
        state.active_profile.icon_name().to_string()
    }

    #[zbus(property)]
    async fn icon_pixmap(&self) -> Vec<(i32, i32, Vec<u8>)> {
        Vec::new()
    }

    #[zbus(property)]
    async fn overlay_icon_name(&self) -> &str {
        ""
    }

    #[zbus(property)]
    async fn overlay_icon_pixmap(&self) -> Vec<(i32, i32, Vec<u8>)> {
        Vec::new()
    }

    #[zbus(property)]
    async fn attention_icon_name(&self) -> &str {
        ""
    }

    #[zbus(property)]
    async fn attention_icon_pixmap(&self) -> Vec<(i32, i32, Vec<u8>)> {
        Vec::new()
    }

    #[zbus(property)]
    async fn attention_movie_name(&self) -> &str {
        ""
    }

    #[zbus(property)]
    async fn tool_tip(&self) -> (String, Vec<(i32, i32, Vec<u8>)>, String, String) {
        let state = self.state.read().await;
        let icon = state.active_profile.icon_name().to_string();
        let title = format!("PostureFlow: {}", state.active_profile.as_str().to_uppercase());
        let desc = format!("{}\nClick to cycle context profile.", state.active_profile.display_name());
        (icon, Vec::new(), title, desc)
    }

    async fn context_menu(&self, _x: i32, _y: i32) {
        // Handled via DBusMenu
    }

    async fn activate(&self, _x: i32, _y: i32) {
        // Left click cycles to the next profile
        let next_profile = {
            let state = self.state.read().await;
            state.active_profile.next()
        };
        let _ = self.action_tx.send(MenuAction::SwitchProfile(next_profile)).await;
    }

    async fn secondary_activate(&self, _x: i32, _y: i32) {
        let _ = self.action_tx.send(MenuAction::ShowStatus).await;
    }

    async fn scroll(&self, delta: i32, _orientation: &str) {
        if delta > 0 {
            let next = {
                let state = self.state.read().await;
                state.active_profile.next()
            };
            let _ = self.action_tx.send(MenuAction::SwitchProfile(next)).await;
        }
    }

    #[zbus(signal)]
    pub async fn new_title(ctxt: &zbus::SignalContext<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn new_icon(ctxt: &zbus::SignalContext<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn new_attention_icon(ctxt: &zbus::SignalContext<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn new_overlay_icon(ctxt: &zbus::SignalContext<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn new_menu(ctxt: &zbus::SignalContext<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn new_tool_tip(ctxt: &zbus::SignalContext<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn new_status(ctxt: &zbus::SignalContext<'_>, status: &str) -> zbus::Result<()>;
}
