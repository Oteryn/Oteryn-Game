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
            mailbox.task_board.feedback = Some(tr!("The session has ended"));
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
            mailbox.task_board.feedback = Some(tr!("The session has ended"));
        }
    }
}

pub(in crate::play) fn enqueue_talisman(
    mailbox: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    intent: oteryn_client_ui::task_board::talisman::Intent,
) -> bool {
    use oteryn_client_ui::task_board::talisman;
    if mailbox.world_input_blocked() {
        return false;
    }
    if let talisman::Intent::Upgrade(path) = intent {
        let Some(level) = mailbox
            .task_board
            .weekly
            .as_ref()
            .and_then(|r| r.talisman.get(usize::from(path)))
        else {
            mailbox.task_board.feedback = Some("Refresh the board and select a talisman path");
            return false;
        };
        if level.level >= 355 {
            mailbox.task_board.feedback = Some("Maximum talisman level reached");
            return false;
        }
    }
    let Some(command) = talisman::request(&mut mailbox.task_board, intent) else {
        return false;
    };
    if commands.send(PlayCommand::Feature(command)).is_err() {
        mailbox.task_board.pending = false;
        mailbox.task_board.feedback = Some("The session has ended");
        return false;
    }
    true
}
impl PlayLink {
    pub fn task_board_talisman_request(
        &self,
        intent: oteryn_client_ui::task_board::talisman::Intent,
    ) -> bool {
        enqueue_talisman(&mut lock(&self.mailbox), &self.commands, intent)
    }
}
