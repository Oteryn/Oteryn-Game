use crate::play::{PlayLink, mailbox::lock};

impl PlayLink {
    /// Consume only transient Bevy presentation events; retained panel history is unchanged.
    #[cfg(feature = "bevy-shell")]
    pub(crate) fn take_achievement_banners(&self) -> std::collections::VecDeque<String> {
        std::mem::take(&mut lock(&self.mailbox).achievement_banners)
    }

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
