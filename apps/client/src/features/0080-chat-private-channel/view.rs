use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::chat_private_channel::ChatPanel;
impl PlayLink {
    #[must_use]
    pub fn chat_private_panel(&self) -> ChatPanel {
        lock(&self.mailbox).chat_private.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::Controls;
