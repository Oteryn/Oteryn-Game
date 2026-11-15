use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::daily_reward::DailyRewardResponse;
impl Outbox {
    pub(in crate::play) fn daily_reward_details_result(
        &self,
        response: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        let mut mailbox = lock(&self.0);
        let claim_feedback = mailbox.daily_reward_claim_feedback.take();
        if oteryn_client_ui::daily_reward::apply_details(&mut mailbox.daily_reward, response)
            && claim_feedback.is_some()
        {
            mailbox.daily_reward.feedback = claim_feedback;
        }
    }

    pub(in crate::play) fn daily_reward_initialize(&self, selected: bool) {
        lock(&self.0).daily_reward.selected = selected;
    }
    pub(in crate::play) fn daily_reward_result(&self, response: &DailyRewardResponse) {
        let mut mailbox = lock(&self.0);
        oteryn_client_ui::daily_reward::apply_result(&mut mailbox.daily_reward, response);
        if matches!(response.status, oteryn_session::CommandStatus::Accepted | oteryn_session::CommandStatus::DuplicateReplay)
            && matches!(response.reply, Some(oteryn_client_runtime::daily_reward::DailyRewardReply::Result(result)) if matches!(result.disposition, 1 | 2))
        {
            mailbox.daily_reward_claim_feedback = mailbox.daily_reward.feedback;
            mailbox.daily_reward_refresh_wall = true;
            mailbox.daily_reward_refresh_details = true;
            mailbox.daily_reward.details_attempted = false;
        }
    }
}

impl Outbox {
    pub(in crate::play) fn returner_initial_request(
        &self,
    ) -> Option<oteryn_client_runtime::raw_feature::FeatureCommand> {
        oteryn_client_ui::daily_reward::request_returner(&mut lock(&self.0).daily_reward, false)
    }
    pub(in crate::play) fn daily_reward_returner_result(
        &self,
        kind: u32,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        let notice = oteryn_client_ui::daily_reward::apply_returner(
            &mut lock(&self.0).daily_reward,
            kind,
            result,
        );
        #[cfg(windows)]
        if !lock(&self.0).shared_frontend
            && notice.is_some()
            && !super::view::notify_returner()
        {
            lock(&self.0).daily_reward.returner_notified = None;
        }
        #[cfg(not(windows))]
        let _ = notice;
    }
    pub(in crate::play) fn returner_unavailable(&self) {
        let mut m = lock(&self.0);
        m.daily_reward.returner.pending = false;
        m.daily_reward.returner_pending = None;
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl Outbox {
    pub(crate) fn bevy_reward_wall(&self, panel: oteryn_client_ui::daily_reward::DailyRewardPanel) {
        lock(&self.0).daily_reward = panel;
    }
}

impl crate::play::mailbox::Mailbox {
    pub(in crate::play) fn daily_reward_follow_up(
        &mut self,
    ) -> Option<crate::combat_input::PlayCommand> {
        use crate::combat_input::PlayCommand;
        use oteryn_client_ui::daily_reward::{Intent, request, request_details};
        if self.world_input_blocked() {
            return None;
        }
        if self.daily_reward_refresh_wall {
            let command = request(&mut self.daily_reward, Intent::Refresh)?;
            self.daily_reward_refresh_wall = false;
            return Some(PlayCommand::DailyReward(command));
        }
        if self.daily_reward_refresh_details {
            let command = request_details(&mut self.daily_reward)?;
            self.daily_reward_refresh_details = false;
            return Some(PlayCommand::Feature(command));
        }
        None
    }
}
