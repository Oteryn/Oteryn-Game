use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::quest_log::QuestPanel;

impl PlayLink {
    /// A consistent projection for the native feature window and recorded-frame tests.
    #[must_use]
    pub fn quest_panel(&self) -> QuestPanel {
        lock(&self.mailbox).quest.clone()
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
