use crate::play::{PlayLink, mailbox::lock};

impl PlayLink {
    pub fn achievement_notices_panel(
        &self,
    ) -> oteryn_client_ui::achievement_notices::AchievementNoticesPanel {
        lock(&self.mailbox).achievement_notices.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
