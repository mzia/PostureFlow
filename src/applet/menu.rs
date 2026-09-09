use std::collections::HashMap;
use tokio::sync::mpsc;
use zbus::interface;
use zbus::zvariant::{Structure, Type, Value};
use crate::profile::Profile;
use crate::applet::state::SharedState;

#[derive(Debug, Clone)]
pub enum MenuAction {
    SwitchProfile(Profile),
    ShowStatus,
    ResetDefaults,
    OpenGui,
    Quit,
}

#[derive(Debug, Type, serde::Serialize)]
pub struct MenuItemNode {
    pub id: i32,
    pub props: HashMap<String, Value<'static>>,
    pub children: Vec<Value<'static>>,
}

impl MenuItemNode {
    pub fn to_value(self) -> Value<'static> {
        let s = Structure::from((self.id, self.props, self.children));
        Value::from(s)
    }
}

pub struct DbusMenu {
    state: SharedState,
    action_tx: mpsc::Sender<MenuAction>,
}

impl DbusMenu {
    pub fn new(state: SharedState, action_tx: mpsc::Sender<MenuAction>) -> Self {
        Self { state, action_tx }
    }

    fn item_props(id: i32, active: Profile) -> Option<HashMap<String, Value<'static>>> {
        let mut props = HashMap::new();
        match id {
            0 => {
                props.insert("children-display".to_string(), Value::from("submenu"));
                Some(props)
            }
            1 => {
                props.insert("label".to_string(), Value::from("Pop! Profile Manager"));
                props.insert("enabled".to_string(), Value::from(false));
                Some(props)
            }
            2 => {
                props.insert(
                    "label".to_string(),
                    Value::from(format!("Active: {}", active.display_name())),
                );
                props.insert("enabled".to_string(), Value::from(false));
                Some(props)
            }
            3 | 20 | 30 => {
                props.insert("type".to_string(), Value::from("separator"));
                Some(props)
            }
            10 => {
                props.insert(
                    "label".to_string(),
                    Value::from("🏠 Home Profile (Balanced Power & Media)"),
                );
                props.insert("toggle-type".to_string(), Value::from("checkmark"));
                props.insert(
                    "toggle-state".to_string(),
                    Value::from(if active == Profile::Home { 1i32 } else { 0i32 }),
                );
                Some(props)
            }
            11 => {
                props.insert(
                    "label".to_string(),
                    Value::from("💼 Work Profile (PowerTOP, Strict FW, VPN)"),
                );
                props.insert("toggle-type".to_string(), Value::from("checkmark"));
                props.insert(
                    "toggle-state".to_string(),
                    Value::from(if active == Profile::Work { 1i32 } else { 0i32 }),
                );
                Some(props)
            }
            12 => {
                props.insert(
                    "label".to_string(),
                    Value::from("💻 Dev Profile (Docker, SSH, High Performance)"),
                );
                props.insert("toggle-type".to_string(), Value::from("checkmark"));
                props.insert(
                    "toggle-state".to_string(),
                    Value::from(if active == Profile::Dev { 1i32 } else { 0i32 }),
                );
                Some(props)
            }
            13 => {
                props.insert(
                    "label".to_string(),
                    Value::from("🔒 Secure Profile (Lockdown, Kill Unused)"),
                );
                props.insert("toggle-type".to_string(), Value::from("checkmark"));
                props.insert(
                    "toggle-state".to_string(),
                    Value::from(if active == Profile::Secure { 1i32 } else { 0i32 }),
                );
                Some(props)
            }
            21 => {
                props.insert(
                    "label".to_string(),
                    Value::from("📊 View Profile & Power Status..."),
                );
                Some(props)
            }
            22 => {
                props.insert(
                    "label".to_string(),
                    Value::from("🔄 Reset to Factory Defaults"),
                );
                Some(props)
            }
            23 => {
                props.insert(
                    "label".to_string(),
                    Value::from("⚙ Configure Profiles & Import..."),
                );
                Some(props)
            }
            31 => {
                props.insert("label".to_string(), Value::from("✕ Quit Applet"));
                Some(props)
            }
            _ => None,
        }
    }
}

#[interface(name = "com.canonical.dbusmenu")]
impl DbusMenu {
    async fn get_layout(
        &self,
        _parent_id: i32,
        _recursion_depth: i32,
        _property_names: Vec<String>,
    ) -> (u32, MenuItemNode) {
        let (active, revision) = {
            let state = self.state.read().await;
            (state.active_profile, state.menu_revision)
        };

        let root_props = Self::item_props(0, active).unwrap_or_default();
        let child_ids = [1, 2, 3, 10, 11, 12, 13, 20, 21, 22, 23, 30, 31];
        let mut children = Vec::with_capacity(child_ids.len());

        for id in child_ids {
            if let Some(props) = Self::item_props(id, active) {
                let node = MenuItemNode {
                    id,
                    props,
                    children: Vec::new(),
                };
                children.push(node.to_value());
            }
        }

        (
            revision,
            MenuItemNode {
                id: 0,
                props: root_props,
                children,
            },
        )
    }

    async fn get_group_properties(
        &self,
        ids: Vec<i32>,
        _property_names: Vec<String>,
    ) -> Vec<(i32, HashMap<String, Value<'static>>)> {
        let active = self.state.read().await.active_profile;
        let mut result = Vec::new();
        for id in ids {
            if let Some(props) = Self::item_props(id, active) {
                result.push((id, props));
            }
        }
        result
    }

    async fn get_property(&self, id: i32, name: String) -> Value<'static> {
        let active = self.state.read().await.active_profile;
        if let Some(props) = Self::item_props(id, active) {
            for (k, v) in props {
                if k == name {
                    return v;
                }
            }
        }
        Value::from("")
    }

    async fn event(
        &self,
        id: i32,
        event_id: String,
        _data: Value<'_>,
        _timestamp: u32,
    ) {
        if event_id == "clicked" {
            let action = match id {
                10 => Some(MenuAction::SwitchProfile(Profile::Home)),
                11 => Some(MenuAction::SwitchProfile(Profile::Work)),
                12 => Some(MenuAction::SwitchProfile(Profile::Dev)),
                13 => Some(MenuAction::SwitchProfile(Profile::Secure)),
                21 => Some(MenuAction::ShowStatus),
                22 => Some(MenuAction::ResetDefaults),
                23 => Some(MenuAction::OpenGui),
                31 => Some(MenuAction::Quit),
                _ => None,
            };

            if let Some(action) = action {
                let _ = self.action_tx.send(action).await;
            }
        }
    }

    async fn event_group(
        &self,
        events: Vec<(i32, String, Value<'_>, u32)>,
    ) -> Vec<i32> {
        for (id, event_id, data, ts) in events {
            self.event(id, event_id, data, ts).await;
        }
        Vec::new()
    }

    async fn about_to_show(&self, _id: i32) -> bool {
        false
    }

    async fn about_to_show_group(
        &self,
        _ids: Vec<i32>,
    ) -> (Vec<i32>, Vec<i32>) {
        (Vec::new(), Vec::new())
    }

    #[zbus(signal)]
    pub async fn layout_updated(
        ctxt: &zbus::SignalContext<'_>,
        revision: u32,
        parent: i32,
    ) -> zbus::Result<()>;
}
