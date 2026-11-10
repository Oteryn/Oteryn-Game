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

impl Outbox {
    pub(in crate::play) fn daily_reward_picker_result(
        &self,
        kind: u32,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        oteryn_client_ui::daily_reward::apply_picker(&mut lock(&self.0).daily_reward, kind, result);
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
        if notice.is_some() && !super::view::notify_returner() {
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
