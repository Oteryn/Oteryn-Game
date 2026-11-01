use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::stash::StashPanel;
impl PlayLink {
    pub fn stash_panel(&self) -> StashPanel {
        lock(&self.mailbox).stash.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
