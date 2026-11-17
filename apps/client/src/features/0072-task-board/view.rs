use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn task_board_set_open(&self, open: bool) {
        if open {
            self.open_task_board();
        } else {
            lock(&self.mailbox).task_board.open = false;
        }
    }
    #[must_use]
    pub fn task_board_panel(&self) -> oteryn_client_ui::task_board::TaskBoardPanel {
        lock(&self.mailbox).task_board.clone()
    }
    pub fn open_task_board(&self) {
        self.task_board_full_request(oteryn_client_ui::task_board::FullIntent::Refresh);
        lock(&self.mailbox).task_board.open = true;
    }
}

impl PlayLink {
    pub fn open_soulseals(&self) {
        self.task_board_full_request(oteryn_client_ui::task_board::FullIntent::Refresh);
        self.soulseal_catalogue_request(0);
        lock(&self.mailbox).task_board.soulseals_open = true;
    }
}
