use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::boss_rewards::BossRewardsResponse;
impl Outbox {
    pub(in crate::play) fn boss_rewards_initialize(&self, selected: bool) {
        lock(&self.0).boss_rewards.selected = selected;
    }
    pub(in crate::play) fn boss_rewards_result(&self, response: &BossRewardsResponse) {
        {
            let mut mailbox = lock(&self.0);
            if oteryn_client_ui::boss_rewards::apply_result(&mut mailbox.boss_rewards, response)
                && response.status == oteryn_session::CommandStatus::Accepted
                && matches!(&response.reply, Some(oteryn_client_runtime::boss_rewards::BossRewardsReply::Collect(collected)) if collected.moved > 0)
            {
                mailbox.combat_capture.reward_collections =
                    mailbox.combat_capture.reward_collections.saturating_add(1);
            }
        }
        self.analyser_boss_rewards_result(response);
    }
}

#[cfg(all(test, bevy_shell))]
impl Outbox {
    /// Applies one server reply through the production reducer for the Bevy form tests.
    pub(crate) fn bevy_boss_rewards_result(&self, response: &BossRewardsResponse) {
        self.boss_rewards_result(response);
    }
}
