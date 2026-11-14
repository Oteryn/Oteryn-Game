use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn task_board_panel(&self) -> oteryn_client_ui::task_board::TaskBoardPanel {
        lock(&self.mailbox).task_board.clone()
    }
    pub fn open_task_board(&self) {
        self.task_board_full_request(oteryn_client_ui::task_board::FullIntent::Refresh);
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).task_board.open = true;
        }
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(windows)]
#[path = "soulseals_window.rs"]
mod soulseals_window;
impl PlayLink {
    pub fn open_soulseals(&self) {
        self.task_board_full_request(oteryn_client_ui::task_board::FullIntent::Refresh);
        self.soulseal_catalogue_request(0);
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            soulseals_window::open(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).task_board.soulseals_open = true;
        }
    }
}

#[cfg(windows)]
#[path = "weekly_results_window.rs"]
pub(in crate::play) mod weekly_results_window;

/// Snapshot-bound controls used by the shared frontend.
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub(crate) enum TaskBoardControl {
    Close,
    Catalogue,
    Request(oteryn_client_ui::task_board::FullIntent),
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn task_board_action_bound(
        &self,
        expected: &oteryn_client_ui::task_board::TaskBoardPanel,
        action: TaskBoardControl,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed()
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || mailbox.task_board != *expected
            || !expected.selected
            || !expected.open
            || expected.pending
        {
            return false;
        }
        match action {
            TaskBoardControl::Catalogue => { super::input::soulseals::send(&mut mailbox, &self.commands, oteryn_client_ui::task_board::fights::FightIntent::Catalogue { after: 0 }); }
            TaskBoardControl::Close => mailbox.task_board.open = false,
            TaskBoardControl::Request(intent) => {
                let Some(command) = oteryn_client_ui::task_board::full_request(
                    &mut mailbox.task_board, intent,
                ) else { return false; };
                if self.commands.send(crate::combat_input::PlayCommand::Feature(command)).is_err() {
                    mailbox.task_board.pending = false;
                    mailbox.task_board.feedback = Some("The session has ended");
                    return false;
                }
            }
        }
        true
    }
}
