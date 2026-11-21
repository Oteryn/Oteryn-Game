use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn depot_search_panel(&self) -> oteryn_client_ui::depot_search::DepotSearchPanel {
        lock(&self.mailbox).depot_search.clone()
    }
}
#[cfg(test)]
#[path = "native.rs"]
pub(in crate::play) mod native;
