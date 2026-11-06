use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn depot_search_panel(&self) -> oteryn_client_ui::depot_search::DepotSearchPanel {
        lock(&self.mailbox).depot_search.clone()
    }
}
#[cfg(any(windows, test))]
#[path = "native.rs"]
pub(in crate::play) mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
