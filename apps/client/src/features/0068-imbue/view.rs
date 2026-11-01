use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::imbue::ImbuePanel;

impl PlayLink {
    #[must_use]
    pub fn imbue_panel(&self) -> ImbuePanel {
        lock(&self.mailbox).imbue.clone()
    }

    pub fn open_imbue(&self) {
        self.imbue_request(oteryn_client_ui::imbue::Intent::Refresh);
        #[cfg(windows)]
        open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).imbue.open = true;
        }
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
