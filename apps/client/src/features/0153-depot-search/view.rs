use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn depot_search_panel(&self) -> oteryn_client_ui::depot_search::DepotSearchPanel {
        lock(&self.mailbox).depot_search.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
