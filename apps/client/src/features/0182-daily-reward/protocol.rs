use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::daily_reward::DailyRewardResponse;
impl Outbox {
    pub(in crate::play) fn daily_reward_details_result(
        &self,
        response: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        oteryn_client_ui::daily_reward::apply_details(&mut lock(&self.0).daily_reward, response);
    }

    pub(in crate::play) fn daily_reward_initialize(&self, selected: bool) {
        lock(&self.0).daily_reward.selected = selected;
    }
    pub(in crate::play) fn daily_reward_result(&self, response: &DailyRewardResponse) {
        oteryn_client_ui::daily_reward::apply_result(&mut lock(&self.0).daily_reward, response);
    }
}
