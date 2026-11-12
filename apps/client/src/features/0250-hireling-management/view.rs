use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn hireling_management_open(&self) {
        #[cfg(windows)]
        native::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).hireling_management.open = true;
            self.hireling_management_request(oteryn_client_ui::hireling_management::Intent::List);
        }
    }
    pub fn hireling_management_panel(&self) -> oteryn_client_ui::hireling_management::Panel {
        lock(&self.mailbox).hireling_management.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
