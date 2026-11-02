use crate::play::{PlayLink, mailbox::lock};

impl PlayLink {
    pub fn animus_panel(&self) -> oteryn_client_ui::animus_mastery::AnimusPanel {
        lock(&self.mailbox).animus.clone()
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
