use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn browse_field_panel(&self) -> oteryn_client_ui::browse_field::BrowseFieldPanel {
        lock(&self.mailbox).browse_field.clone()
    }
}
#[cfg(any(windows, feature = "bevy-shell"))]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "bevy_tests.rs"]
pub(in crate::play) mod bevy_tests;

#[cfg(feature = "bevy-shell")]
#[path = "view/bound.rs"]
mod bound;
#[cfg(feature = "bevy-shell")]
pub(crate) use bound::{Action as BrowseFieldAction, Binding as BrowseFieldBinding};
