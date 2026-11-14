use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn hireling_management_open(&self) {
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            native::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
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

#[cfg(feature = "bevy-shell")]
#[path = "view/bound.rs"]
mod bound;
#[cfg(feature = "bevy-shell")]
pub(crate) use bound::{HirelingAction, HirelingRetry, HirelingReview, HirelingSnapshot};
