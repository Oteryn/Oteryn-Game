use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::imbue::ImbuePanel;

impl PlayLink {
    #[must_use]
    pub fn imbue_panel(&self) -> ImbuePanel {
        lock(&self.mailbox).imbue.clone()
    }

    pub fn open_imbue(&self) {
        self.imbue_request(oteryn_client_ui::imbue::Intent::Refresh);
        lock(&self.mailbox).imbue.open = true;
    }
}


impl PlayLink {
    pub fn open_imbue_tracker(&self) {
        if lock(&self.mailbox).imbue.tracker_open {
            return;
        }
        lock(&self.mailbox).imbue.tracker_open = true;
        self.imbue_tracker_refresh();
    }
}
