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
