use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn item_forge_panel(&self) -> oteryn_client_ui::item_forge::ItemForgePanel {
        lock(&self.mailbox).item_forge.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
