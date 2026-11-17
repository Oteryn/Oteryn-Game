use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::boss_rewards::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn boss_rewards_request(&self, intent: Intent) {
        enqueue(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn boss_rewards_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::boss_rewards::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).boss_rewards.open = true;
        }
        true
    }
}
pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    if let Some(command) = request(&mut mailbox.boss_rewards, intent)
        && commands.send(PlayCommand::BossRewards(command)).is_err()
    {
        mailbox.boss_rewards.pending = false;
        mailbox.boss_rewards.feedback = Some(tr!("The session has ended"));
    }
}
