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

impl PlayLink {
    pub fn open_daily_reward_picker(&self) {
        self.daily_reward_picker_request(None);
        #[cfg(windows)]
        picker_native::open_picker(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).daily_reward.picker_open = true;
        }
    }
}
#[cfg(windows)]
#[path = "picker_native.rs"]
mod picker_native;
