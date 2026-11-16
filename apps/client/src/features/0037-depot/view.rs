use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::depot::{DepotPanel, Intent};
impl PlayLink {
    pub fn depot_panel(&self) -> DepotPanel {
        lock(&self.mailbox).depot.clone()
    }
    pub fn open_depot(&self) {
        self.depot_request(Intent::Browse {
            box_number: 1,
            page: 0,
        });
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            native::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).depot.open = true;
        }
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;

/// Shared frontend actions are bound to the exact displayed inventory and depot page.
#[cfg(bevy_shell)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DepotUiAction {
    Close,
    Request(Intent),
}
#[cfg(bevy_shell)]
impl PlayLink {
    pub(crate) fn depot_action_bound(&self, expected: &DepotPanel, action: DepotUiAction) {
        let mut mailbox = lock(&self.mailbox);
        if !mailbox.depot.open || &mailbox.depot != expected {
            return;
        }
        match action {
            DepotUiAction::Close => mailbox.depot.open = false,
            DepotUiAction::Request(intent) => {
                super::input::enqueue(&mut mailbox, &self.commands, intent)
            }
        }
    }
}
