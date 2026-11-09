use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::daily_reward::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn daily_reward_details_request(&self) {
        enqueue_details(&mut lock(&self.mailbox), &self.commands);
    }

    pub fn daily_reward_request(&self, intent: Intent) {
        enqueue(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn daily_reward_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::daily_reward::open_key(events) {
            return false;
        }
        self.daily_reward_request(Intent::Refresh);
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).daily_reward.open = true;
        }
        true
    }
}
pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    if mailbox.world_input_blocked() {
        return;
    }
    if let Some(command) = request(&mut mailbox.daily_reward, intent)
        && commands.send(PlayCommand::DailyReward(command)).is_err()
    {
        mailbox.daily_reward.pending = false;
        mailbox.daily_reward.feedback = Some(tr!("The session has ended"));
    }
}

pub(in crate::play) fn enqueue_details(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
) {
    if mailbox.world_input_blocked() {
        return;
    }
    if let Some(command) =
        oteryn_client_ui::daily_reward::request_details(&mut mailbox.daily_reward)
        && commands.send(PlayCommand::Feature(command)).is_err()
    {
        mailbox.daily_reward.pending = false;
        mailbox.daily_reward.details_pending = false;
        mailbox.daily_reward.feedback = Some("The session has ended");
    }
}
