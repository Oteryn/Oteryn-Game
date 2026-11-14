use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn item_forge_panel(&self) -> oteryn_client_ui::item_forge::ItemForgePanel {
        lock(&self.mailbox).item_forge.clone()
    }
}
#[cfg(any(windows, test))]
#[path = "native.rs"]
pub(in crate::play) mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(windows)]
pub(in crate::play) fn poll_resource_dialog(
    mailbox: &crate::play::mailbox::SharedMailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
) {
    native::open_resource_dialog(mailbox, commands);
}

#[cfg(feature = "bevy-shell")]
#[path = "bound.rs"]
mod bound;
#[cfg(feature = "bevy-shell")]
pub(crate) use bound::{ForgeControl, ForgeSnapshot};
