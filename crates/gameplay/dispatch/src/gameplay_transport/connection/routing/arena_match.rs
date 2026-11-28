use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::arena::{self, ArenaAction, ArenaDisposition};
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    if arena::decode_match_request(command.payload).is_err() {
        return Some(Dispatch::Unregistered);
    }
    authority
        .arena_match_command(*admitted, command.command_id, command.payload)
        .await
        .map(|bytes| Dispatch::Reply(CommandStatus::Accepted, bytes))
}

/// A successful Svargrond Enter has a committed fee (or its replay receipt).
pub(in super::super) fn is_committed_entry(command: ClientCommandView<'_>, payload: &[u8]) -> bool {
    command.command_type == arena::COMMAND_TYPE_ARENA_MATCH
        && arena::decode_match_request(command.payload) == Ok(ArenaAction::Enter)
        && arena::decode_match_result(payload)
            .is_ok_and(|result| result.disposition == ArenaDisposition::Applied)
}
