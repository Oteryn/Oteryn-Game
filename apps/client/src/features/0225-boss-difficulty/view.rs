use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::boss_difficulty::BossDifficultyPanel;
impl PlayLink {
    #[must_use]
    pub fn boss_difficulty_panel(&self) -> BossDifficultyPanel {
        lock(&self.mailbox).boss_difficulty.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
