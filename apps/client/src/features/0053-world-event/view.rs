use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::world_event::WorldEventPanel;
impl PlayLink {
    pub fn world_event_panel(&self) -> WorldEventPanel {
        lock(&self.mailbox).world_event.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
