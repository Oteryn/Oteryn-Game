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

impl crate::play::Outbox {
    pub(in crate::play) fn task_board_full_result(
        &self,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        oteryn_client_ui::task_board::apply_full_result(&mut lock(&self.0).task_board, result);
    }
}
