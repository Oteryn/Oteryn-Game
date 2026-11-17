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
            mailbox.loyalty_skills.error = Some(tr!("Session ended"));
        }
    }

    pub fn skills_widget_request(&self) {
        self.loyalty_skills_request();
        self.skill_progress_request();
    }

    pub fn loyalty_skills_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::loyalty_skills::open_key(events) {
            return false;
        }
        self.skills_widget_request();
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).loyalty_skills.open = true;
        }
        true
    }
}
