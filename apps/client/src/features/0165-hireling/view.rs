use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::hireling::HirelingPanel;

impl PlayLink {
    #[must_use]
    pub fn hireling_panel(&self) -> HirelingPanel {
        lock(&self.mailbox).hireling.clone()
    }

    #[cfg(windows)]
    pub fn open_hireling_panel(&self) {
        native::open_window(self);
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
