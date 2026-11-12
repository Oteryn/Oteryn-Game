use super::{Panel, Pending, TerminalReplies};
use oteryn_client_runtime::{
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
    server_modal::{
        Answer, CAPABILITY_SERVER_MODAL_V1, COMMAND_TYPE_SERVER_MODAL_ANSWER,
        COMMAND_TYPE_SERVER_MODAL_QUERY, QUERY_WITH_TERMINAL_REPLY, Reply,
    },
};
pub fn hireling_store(
    payload: Vec<u8>,
) -> oteryn_client_runtime::server_mechanic::ServerMechanicCommand {
    oteryn_client_runtime::server_mechanic::ServerMechanicCommand {
        capability: CAPABILITY_SERVER_MODAL_V1,
        command_type: oteryn_client_runtime::server_modal::COMMAND_TYPE_HIRELING_STORE_SELECTION,
        payload,
    }
}
pub fn query(panel: &mut Panel) -> Option<FeatureCommand> {
    if !panel.selected || panel.logged_out || panel.pending.is_some() || panel.active.is_some() {
        return None;
    }
    panel.pending = Some(Pending::Query);
    panel.feedback = None;
    Some(FeatureCommand {
        capability: CAPABILITY_SERVER_MODAL_V1,
        kind: COMMAND_TYPE_SERVER_MODAL_QUERY,
        payload: if panel.terminal_replies == TerminalReplies::Unsupported {
            Vec::new()
        } else {
            QUERY_WITH_TERMINAL_REPLY.to_vec()
        },
    })
}
pub fn answer(panel: &mut Panel, answer: Answer) -> Option<FeatureCommand> {
    if !panel.selected
        || panel.logged_out
        || panel.pending.is_some()
        || !panel.active.as_ref()?.accepts(answer)
    {
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
        if pending == Pending::Query
            && panel.terminal_replies == TerminalReplies::Probing
            && result.status == CommandStatus::Rejected
        {
            panel.terminal_replies = TerminalReplies::Unsupported;
        }
        panel.feedback = Some("The server refused the dialog action; retry or cancel");
        return false;
    }
    match (pending, Reply::decode(&result.payload)) {
        (Pending::Query, Ok(reply @ (Reply::Dialog(_) | Reply::None))) => {
            panel.active = if let Reply::Dialog(dialog) = reply {
                Some(dialog)
            } else {
                None
            };
            if result.status == CommandStatus::Accepted
                && panel.terminal_replies == TerminalReplies::Probing
            {
                panel.terminal_replies = TerminalReplies::Enabled;
            }
            panel.feedback = None;
            true
        }
        (Pending::Answer(sent), Ok(Reply::Answered(received))) if sent == received => {
            panel.active = None;
            panel.feedback = None;
            true
        }
        (Pending::Answer(sent), Ok(Reply::AnsweredAndLoggedOut(received)))
            if sent == received
                && result.status == CommandStatus::Accepted
                && panel.terminal_replies == TerminalReplies::Enabled
                && panel.active.as_ref().is_some_and(|dialog| {
                    dialog.accepts(sent) && dialog.cancel_button != Some(sent.button_id)
                }) =>
        {
            panel.active = None;
            panel.feedback = None;
            panel.logged_out = true;
            true
        }
        _ => {
            panel.feedback = Some("Invalid server dialog reply");
            false
        }
    }
}
