use std::sync::Arc;
use tokio::sync::RwLock;
use crate::profile::Profile;

#[derive(Debug, Clone)]
pub struct AppState {
    pub active_profile: Profile,
    pub menu_revision: u32,
    pub cached_status: Option<String>,
}

impl AppState {
    pub fn new(initial_profile: Profile) -> Self {
        Self {
            active_profile: initial_profile,
            menu_revision: 1,
            cached_status: None,
        }
    }

    pub fn set_profile(&mut self, profile: Profile) -> bool {
        if self.active_profile != profile {
            self.active_profile = profile;
            self.menu_revision = self.menu_revision.wrapping_add(1);
            true
        } else {
            false
        }
    }
}

pub type SharedState = Arc<RwLock<AppState>>;
