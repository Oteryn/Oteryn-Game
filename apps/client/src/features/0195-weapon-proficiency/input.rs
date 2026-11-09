use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::weapon_proficiency::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn weapon_proficiency_select_weapon(&self, row: usize) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.weapon_proficiency.select_weapon(row)
            && let Some(command) = request(&mut mailbox.weapon_proficiency, Intent::State)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.weapon_proficiency.pending = false;
            mailbox.weapon_proficiency.feedback = Some(tr!("The session has ended"));
        }
    }
    pub fn weapon_proficiency_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.weapon_proficiency, intent)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.weapon_proficiency.pending = false;
            mailbox.weapon_proficiency.feedback = Some(tr!("The session has ended"));
        }
    }
    pub fn weapon_proficiency_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::weapon_proficiency::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).weapon_proficiency.open = true;
        }
        self.weapon_proficiency_request(Intent::Catalogue);
        true
    }
}
