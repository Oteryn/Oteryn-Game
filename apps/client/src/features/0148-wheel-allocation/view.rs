use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::wheel_allocation::WheelPanel;

impl PlayLink {
    #[must_use]
    pub fn wheel_panel(&self) -> WheelPanel {
        lock(&self.mailbox).wheel.clone()
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
