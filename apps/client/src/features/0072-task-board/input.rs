use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::task_board::{Intent, request};
impl PlayLink {
    pub fn task_board_input(&self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
        if oteryn_client_ui::task_board::soulseals_key(events) {
            self.open_soulseals();
            return true;
        }
        if !oteryn_client_ui::task_board::open_key(events) {
            return false;
        }
        self.open_task_board();
        true
    }

    pub fn task_board_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.task_board, intent)
            && self.commands.send(PlayCommand::TaskBoard(command)).is_err()
        {
            mailbox.task_board.pending = false;
            mailbox.task_board.feedback = Some("The session has ended");
        }
    }
}

impl PlayLink {
    pub fn task_board_full_request(&self, intent: oteryn_client_ui::task_board::FullIntent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) =
            oteryn_client_ui::task_board::full_request(&mut mailbox.task_board, intent)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.task_board.pending = false;
            mailbox.task_board.feedback = Some("The session has ended");
        }
    }
}
