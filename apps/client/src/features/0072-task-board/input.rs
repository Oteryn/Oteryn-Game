use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::task_board::{Intent, request};
impl PlayLink {
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
