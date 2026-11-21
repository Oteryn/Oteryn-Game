use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::daily_reward::DailyRewardPanel;
impl PlayLink {
    #[must_use]
    pub fn daily_reward_panel(&self) -> DailyRewardPanel {
        lock(&self.mailbox).daily_reward.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

impl PlayLink {
    pub fn open_returner_reward(&self) {
        self.returner_reward_request(false);
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            returner_native::open_reward(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).daily_reward.returner_open = true;
        }
    }
}
#[cfg(windows)]
#[path = "returner_native.rs"]
mod returner_native;
#[cfg(windows)]
pub(in crate::play) use returner_native::notify as notify_returner;
