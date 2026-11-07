use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn exiva_panel(&self) -> oteryn_client_ui::exiva_settings::ExivaPanel {
        lock(&self.mailbox).exiva.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
