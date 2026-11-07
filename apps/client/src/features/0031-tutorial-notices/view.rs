use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn notices_panel(&self) -> oteryn_client_ui::tutorial_notices::NoticePanel {
        lock(&self.mailbox).notices.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
