use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::highscores::{self, Action};
impl PlayLink {
    pub fn highscores_request(&self, action: Action) {
        let mut mailbox = lock(&self.mailbox);
        let command = if mailbox.highscores.catalogue_selected {
            oteryn_client_ui::highscores_catalogue::request(&mut mailbox.highscores, action)
        } else {
            highscores::request(&mut mailbox.highscores, action)
        };
        if let Some(command) = command
            && self
                .commands
                .send(PlayCommand::ServerMechanic(command))
                .is_err()
        {
            mailbox.highscores.pending = false;
            mailbox.highscores.feedback = Some(tr!("The session has ended").into());
        }
    }
    pub fn highscores_input(&self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
        if !highscores::open_key(events) {
            return false;
        }
        self.highscores_request(Action::Refresh);
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).highscores.open = true;
        }
        true
    }
}
