use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn browse_field_panel(&self) -> oteryn_client_ui::browse_field::BrowseFieldPanel {
        lock(&self.mailbox).browse_field.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
