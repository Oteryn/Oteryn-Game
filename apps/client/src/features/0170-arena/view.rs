use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::arena::ArenaPanel;
impl PlayLink {
    pub fn arena_panel(&self) -> ArenaPanel {
        lock(&self.mailbox).arena.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
