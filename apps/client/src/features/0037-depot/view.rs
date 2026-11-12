use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::depot::{DepotPanel, Intent};
impl PlayLink {
    pub fn depot_panel(&self) -> DepotPanel {
        lock(&self.mailbox).depot.clone()
    }
    pub fn open_depot(&self) {
        self.depot_request(Intent::Browse {
            box_number: 1,
            page: 0,
        });
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            native::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).depot.open = true;
        }
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
