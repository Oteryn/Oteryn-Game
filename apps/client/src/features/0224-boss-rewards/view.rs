use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::boss_rewards::BossRewardsPanel;
impl PlayLink {
    #[must_use]
    pub fn boss_rewards_panel(&self) -> BossRewardsPanel {
        lock(&self.mailbox).boss_rewards.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
