pub mod state;
pub mod client;
pub mod menu;
pub mod sni;
pub mod pixmap;

pub use state::{AppState, SharedState};
pub use client::DaemonClient;
pub use menu::{DbusMenu, MenuAction};
pub use sni::StatusNotifierItem;

pub const SNI_OBJECT_PATH: &str = "/StatusNotifierItem";
pub const MENU_OBJECT_PATH: &str = "/com/canonical/dbusmenu";
pub const WATCHER_BUS_NAME: &str = "org.kde.StatusNotifierWatcher";
pub const WATCHER_OBJECT_PATH: &str = "/StatusNotifierWatcher";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::Profile;

    #[test]
    fn test_app_state_profile_switch() {
        let mut state = AppState::new(Profile::Home);
        assert_eq!(state.active_profile, Profile::Home);
        assert_eq!(state.menu_revision, 1);

        let changed = state.set_profile(Profile::Work);
        assert!(changed);
        assert_eq!(state.active_profile, Profile::Work);
        assert_eq!(state.menu_revision, 2);

        let changed_again = state.set_profile(Profile::Work);
        assert!(!changed_again);
        assert_eq!(state.menu_revision, 2);
    }

    #[test]
    fn test_profile_cycling_loop() {
        let p1 = Profile::Home;
        let p2 = p1.next();
        let p3 = p2.next();
        let p4 = p3.next();
        let p5 = p4.next();

        assert_eq!(p2, Profile::Work);
        assert_eq!(p3, Profile::Dev);
        assert_eq!(p4, Profile::Travel);
        assert_eq!(p5, Profile::Home);
    }
}
