use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::chat::ChatPanel;

impl PlayLink {
    #[must_use]
    pub fn chat_panel(&self) -> ChatPanel {
        lock(&self.mailbox).chat.clone()
    }
}


#[cfg(test)]
#[path = "console_tests.rs"]
mod console_tests;

#[path = "console.rs"]
mod console;

#[cfg(test)]
#[path = "filter_tests.rs"]
mod filter_tests;

#[path = "lists.rs"]
pub(in crate::play) mod lists;
