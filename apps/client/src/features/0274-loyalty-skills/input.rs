use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_input_actions::NormalizedInputEvent;

impl PlayLink {
    pub fn loyalty_skills_request(&self) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(payload) =
            oteryn_client_ui::loyalty_skills::request(&mut mailbox.loyalty_skills)
            && self
                .commands
                .send(PlayCommand::LoyaltySkills(payload))
                .is_err()
        {
            mailbox.loyalty_skills.pending = false;
            mailbox.loyalty_skills.error = Some("Session ended");
        }
    }

    pub fn loyalty_skills_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::loyalty_skills::open_key(events) {
            return false;
        }
        self.loyalty_skills_request();
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).loyalty_skills.open = true;
        }
        true
    }
}
