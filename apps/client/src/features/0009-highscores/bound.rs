//! Atomic presentation actions over the existing highscores owner.
use crate::{
    combat_input::PlayCommand,
    play::{PlayLink, mailbox::lock},
};
use oteryn_client_ui::highscores::{self, Action, HighscoresPanel};
impl PlayLink {
    pub(crate) fn highscores_open_bound(&self, expected: &HighscoresPanel) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.highscores_shutdown
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
            || mailbox.highscores != *expected
            || !expected.selected
        {
            return false;
        }
        mailbox.highscores.open = true;
        true
    }
    pub(crate) fn highscores_close_bound(&self, expected: &HighscoresPanel) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.highscores_shutdown
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
            || mailbox.highscores != *expected
            || !expected.open
        {
            return false;
        }
        mailbox.highscores.open = false;
        true
    }
    pub(crate) fn highscores_request_bound(
        &self,
        expected: &HighscoresPanel,
        action: Action,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.highscores_shutdown
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
            || mailbox.highscores != *expected
            || !expected.open
            || !expected.selected
            || expected.pending
        {
            return false;
        }
        let mut prepared = expected.clone();
        let Some(command) = highscores::request(&mut prepared, action) else {
            return false;
        };
        if self
            .commands
            .send(PlayCommand::ServerMechanic(command))
            .is_err()
        {
            return false;
        }
        mailbox.highscores = prepared;
        true
    }
}
