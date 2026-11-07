use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::battle::{self, Action};
impl PlayLink {
    pub fn battle_request(&self, action: Action) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = battle::request(&mut mailbox.battle, action)
            && self.commands.send(PlayCommand::Battle(command)).is_err()
        {
            mailbox.battle.pending = false;
            mailbox.battle.pending_command = None;
        }
    }
    pub fn battle_input(&self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
        if !battle::toggle_key(events) {
            return false;
        }
        {
            let mut mailbox = lock(&self.mailbox);
            if mailbox.battle.shutdown {
                return true;
            }
            mailbox.battle.open = !mailbox.battle.open;
            if !mailbox.battle.open {
                return true;
            }
        }
        #[cfg(windows)]
        super::view::open_window(self);
        true
    }
}
