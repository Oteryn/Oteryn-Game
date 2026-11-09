use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn task_board_initialize(&self, selected: bool) {
        let mut m = lock(&self.0);
        m.task_board.selected = selected;
        m.task_board.fights = Default::default();
    }
    pub(in crate::play) fn task_board_result(
        &self,
        response: &oteryn_client_runtime::task_board::TaskBoardResponse,
    ) {
        let mut mailbox = lock(&self.0);
        oteryn_client_ui::task_board::apply_result(&mut mailbox.task_board, response);
        let confirmed = matches!(
            response.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        ) && !response.reply.refused;
        crate::play::feature_0010_analyser::protocol::task_progress(&mut mailbox, confirmed);
    }
}

impl crate::play::Outbox {
    pub(in crate::play) fn task_board_full_result(
        &self,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        let mut mailbox = lock(&self.0);
        oteryn_client_ui::task_board::apply_full_result(&mut mailbox.task_board, result);
        let confirmed = matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        ) && mailbox.task_board.feedback.is_none();
        crate::play::feature_0010_analyser::protocol::task_progress(&mut mailbox, confirmed);
    }
}

impl crate::play::Outbox {
    pub(in crate::play) fn soulseal_result(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        oteryn_client_ui::task_board::fights::apply(&mut lock(&self.0).task_board, command, result);
    }
}
