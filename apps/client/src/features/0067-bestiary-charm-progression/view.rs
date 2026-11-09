#[cfg(windows)]
#[path = "catalogue_window.rs"]
mod catalogue_window;
use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn charm_progression_panel(&self) -> oteryn_client_ui::charm_progression::Panel {
        lock(&self.mailbox).properties.charm_progression.clone()
    }
}
#[cfg(windows)]
#[path = "selector_window.rs"]
mod selector_window;
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    catalogue_window::open(link);
    selector_window::open(link);
}
