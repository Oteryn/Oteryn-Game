use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::highscores::{self, Action};
impl PlayLink {
    pub fn highscores_request(&self, action: Action) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = highscores::request(&mut mailbox.highscores, action)
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
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).highscores.open = true;
        }
        true
    }
}
