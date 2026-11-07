use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::daily_reward::DailyRewardPanel;
impl PlayLink {
    #[must_use]
    pub fn daily_reward_panel(&self) -> DailyRewardPanel {
        lock(&self.mailbox).daily_reward
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
