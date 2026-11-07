use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::cyclopedia_tracker::CyclopediaTrackerPanel;
impl PlayLink {
    #[must_use]
    pub fn cyclopedia_tracker_panel(&self) -> CyclopediaTrackerPanel {
        lock(&self.mailbox).cyclopedia_tracker.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
