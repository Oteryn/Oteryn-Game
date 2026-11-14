use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::prey::PreyPanel;

impl PlayLink {
    #[must_use]
    pub fn prey_panel(&self) -> PreyPanel {
        lock(&self.mailbox).prey.clone()
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub fn prey_dispatch_bound(&self, expected: &PreyPanel, intent: Option<oteryn_client_ui::prey::Intent>) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.prey_shutdown || mailbox.world_input_blocked() || mailbox.prey != *expected || !expected.open || !expected.selected || expected.pending.is_some() { return false; }
        if let Some(intent) = intent {
            super::input::enqueue(&mut mailbox, &self.commands, intent);
        } else { mailbox.prey.open = false; }
        true
    }
}

#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub fn prey_choose_bound(&self, expected: &PreyPanel, slot: u8, index: u8) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.prey_shutdown || mailbox.world_input_blocked() || mailbox.prey != *expected || !expected.open { return false; }
        mailbox.prey.choose_candidate(slot, index)
    }
}
