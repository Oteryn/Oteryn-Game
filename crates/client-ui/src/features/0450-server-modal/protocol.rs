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
    answer_with(panel, answer, &crate::server_modal_input::Entry::default())
}
/// Answer with the value the player entered; the entry is used only with capability 490.
pub fn answer_with(
    panel: &mut Panel,
    answer: Answer,
    entry: &crate::server_modal_input::Entry,
) -> Option<FeatureCommand> {
    if !panel.selected
        || panel.logged_out
        || panel.pending.is_some()
        || !panel.active.as_ref()?.accepts(answer)
    {
        return None;
    }
    let cancel = panel.active.as_ref()?.cancel_button == Some(answer.button_id);
    let payload = crate::server_modal_input::answer_payload(
        panel.input,
        panel.active_input.as_ref(),
        cancel,
        answer,
        entry,
    )?;
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
    let decoded = crate::server_modal_input::decode_reply(panel.input, &result.payload);
    let (reply, input) = match decoded {
        Ok((reply, input)) => (Ok(reply), input),
        Err(error) => (Err(error), None),
    };
    match (pending, reply) {
        (Pending::Query, Ok(reply @ (Reply::Dialog(_) | Reply::None))) => {
            panel.active_input = input;
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
            panel.active_input = None;
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
            panel.active_input = None;
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
