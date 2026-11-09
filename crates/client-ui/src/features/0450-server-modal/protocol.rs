use super::{Panel, Pending};
use oteryn_client_runtime::{
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
    server_modal::{
        Answer, CAPABILITY_SERVER_MODAL_V1, COMMAND_TYPE_SERVER_MODAL_ANSWER,
        COMMAND_TYPE_SERVER_MODAL_QUERY, Reply,
    },
};
pub fn query(panel: &mut Panel) -> Option<FeatureCommand> {
    if !panel.selected || panel.pending.is_some() || panel.active.is_some() {
        return None;
    }
    panel.pending = Some(Pending::Query);
    panel.feedback = None;
    Some(FeatureCommand {
        capability: CAPABILITY_SERVER_MODAL_V1,
        kind: COMMAND_TYPE_SERVER_MODAL_QUERY,
        payload: Vec::new(),
    })
}
pub fn answer(panel: &mut Panel, answer: Answer) -> Option<FeatureCommand> {
    if !panel.selected || panel.pending.is_some() || !panel.active.as_ref()?.accepts(answer) {
        return None;
    }
    let payload = answer.encode().ok()?;
    panel.pending = Some(Pending::Answer(answer));
    panel.feedback = None;
    Some(FeatureCommand {
        capability: CAPABILITY_SERVER_MODAL_V1,
        kind: COMMAND_TYPE_SERVER_MODAL_ANSWER,
        payload,
    })
}
pub fn apply_result(panel: &mut Panel, result: &GatedCommandResult) -> bool {
    let Some(pending) = panel.pending.take() else {
        return false;
    };
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some("The server refused the dialog action; retry or cancel");
        return false;
    }
    match (pending, Reply::decode(&result.payload)) {
        (Pending::Query, Ok(Reply::Dialog(dialog))) => {
            panel.active = Some(dialog);
            panel.feedback = None;
            true
        }
        (Pending::Query, Ok(Reply::None)) => {
            panel.active = None;
            panel.feedback = None;
            true
        }
        (Pending::Answer(sent), Ok(Reply::Answered(received))) if sent == received => {
            panel.active = None;
            panel.feedback = None;
            true
        }
        _ => {
            panel.feedback = Some("Invalid server dialog reply");
            false
        }
    }
}
