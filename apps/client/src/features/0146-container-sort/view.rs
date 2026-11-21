use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn container_sort_panel(&self) -> oteryn_client_ui::container_sort::Panel {
        lock(&self.mailbox).properties.container_sort.clone()
    }
}
