use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::party::PartyPanel;

impl PlayLink {
    #[must_use]
    pub fn party_panel(&self) -> PartyPanel {
        lock(&self.mailbox).party.clone()
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
