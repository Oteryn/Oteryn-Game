use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::daily_reward::DailyRewardPanel;
impl PlayLink {
    #[cfg(bevy_shell)]
    pub(crate) fn reward_reset_deadline(&self) -> Option<std::time::Instant> {
        let mailbox = lock(&self.mailbox);
        let (epoch, deadline) = mailbox.reward_reset?;
        mailbox.daily_reward.details.as_ref().filter(|details| details.epoch == epoch).map(|_| deadline)
    }
    #[must_use]
    pub fn daily_reward_panel(&self) -> DailyRewardPanel {
        lock(&self.mailbox).daily_reward.clone()
    }
}

impl PlayLink {
    pub fn open_returner_reward(&self) {
        self.returner_reward_request(false);
        lock(&self.mailbox).daily_reward.returner_open = true;
    }
}

pub(in crate::play) fn returner_form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use crate::play::feature_0115_mail::view::form::Form;
    Form {
        title: tr!("Welcome-back reward"),
        labels: &[],
        buttons: &["Refresh reward", "Claim gift"],
        snapshot: |m| {
            (
                m.daily_reward.returner_open,
                m.daily_reward.returner.pending,
                vec![],
                m.daily_reward.returner.notification(),
            )
        },
        set_open: |m, value| m.daily_reward.returner_open = value,
        submit: |m, commands, action, _| {
            super::input::enqueue_returner(m, commands, action == 1)
        },
    }
}
