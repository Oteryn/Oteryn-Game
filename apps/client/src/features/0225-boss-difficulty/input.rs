use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::boss_difficulty::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn boss_difficulty_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.boss_difficulty, intent)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.boss_difficulty.pending = false;
            mailbox.boss_difficulty.feedback = Some(tr!("The session has ended"));
        }
    }
    pub fn boss_difficulty_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::boss_difficulty::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).boss_difficulty.open = true;
        }
        true
    }
}
