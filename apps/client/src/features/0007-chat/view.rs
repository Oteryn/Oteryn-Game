use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::chat::ChatPanel;

impl PlayLink {
    #[must_use]
    pub fn chat_panel(&self) -> ChatPanel {
        lock(&self.mailbox).chat.clone()
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::{open_shared, open_window};

#[cfg(test)]
#[path = "console_tests.rs"]
mod console_tests;

#[path = "console.rs"]
mod console;
