use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    if oteryn_protocol_oteryn::arena::decode_match_request(command.payload).is_err() {
        return Some(Dispatch::Unregistered);
    }
    authority
        .arena_match_command(*admitted, command.command_id, command.payload)
        .await
        .map(|bytes| Dispatch::Reply(CommandStatus::Accepted, bytes))
}
