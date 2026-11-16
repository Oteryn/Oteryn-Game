use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::boss_rewards::BossRewardsResponse;
impl Outbox {
    pub(in crate::play) fn boss_rewards_initialize(&self, selected: bool) {
        lock(&self.0).boss_rewards.selected = selected;
    }
    pub(in crate::play) fn boss_rewards_result(&self, response: &BossRewardsResponse) {
        oteryn_client_ui::boss_rewards::apply_result(&mut lock(&self.0).boss_rewards, response);
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
