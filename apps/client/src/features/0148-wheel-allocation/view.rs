use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::wheel_allocation::WheelPanel;

impl PlayLink {
    #[must_use]
    pub fn wheel_panel(&self) -> WheelPanel {
        lock(&self.mailbox).wheel.clone()
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[path = "presets.rs"]
pub(in crate::play) mod presets;

#[cfg(windows)]
#[path = "gem_native.rs"]
pub(in crate::play) mod gem_native;
#[cfg(windows)]
pub(in crate::play) fn poll_gems(
    mailbox: &crate::play::mailbox::SharedMailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
) {
    let review = lock(mailbox).wheel.gems.review.is_some();
    if review {
        gem_native::open(mailbox, commands);
    }
}
#[cfg(any(windows, test))]
#[path = "gem_surface.rs"]
pub(in crate::play) mod gem_surface;

#[cfg(windows)]
#[path = "wheel_native.rs"]
pub(in crate::play) mod wheel_native;
#[cfg(any(windows, test))]
#[path = "wheel_surface.rs"]
pub(in crate::play) mod wheel_surface;
