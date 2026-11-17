use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::account_achievements::AchievementsPanel;
impl PlayLink {
    #[must_use]
    pub fn achievements_panel(&self) -> AchievementsPanel {
        lock(&self.mailbox).achievements.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::{open_character, open_window};
