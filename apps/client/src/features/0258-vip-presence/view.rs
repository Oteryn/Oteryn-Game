use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn vip_presence_panel(&self) -> oteryn_client_ui::vip_presence::VipPresencePanel {
        lock(&self.mailbox).vip_presence.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
