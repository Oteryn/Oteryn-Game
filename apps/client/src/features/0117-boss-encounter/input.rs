use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::boss_encounter::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn boss_encounter_request(&self, intent: Intent) {
        enqueue(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn boss_encounter_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::boss_encounter::open_key(events) {
            return false;
        }
        self.boss_encounter_request(Intent::Refresh);
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).boss_encounter.open = true;
        }
        true
    }
}
pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    if let Some(command) = request(&mut mailbox.boss_encounter, intent)
        && commands.send(PlayCommand::Feature(command)).is_err()
    {
        mailbox.boss_encounter.pending = false;
        mailbox.boss_encounter.feedback = Some("The session has ended");
    }
}
