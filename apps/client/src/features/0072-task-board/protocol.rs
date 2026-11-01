use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn task_board_initialize(&self, selected: bool) {
        lock(&self.0).task_board.selected = selected;
    }
    pub(in crate::play) fn task_board_result(
        &self,
        response: &oteryn_client_runtime::task_board::TaskBoardResponse,
    ) {
        oteryn_client_ui::task_board::apply_result(&mut lock(&self.0).task_board, response);
    }
}
