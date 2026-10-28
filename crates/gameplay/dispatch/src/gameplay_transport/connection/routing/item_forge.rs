use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, item_forge::Request};
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let Some(request) = Request::decode(command.payload) else {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    };
    let bytes = authority
        .item_forge(*admitted, command.command_id, request)
        .await?;
    Some(Dispatch::Reply(
        if bytes.is_empty() {
            CommandStatus::Rejected
        } else {
            CommandStatus::Accepted
        },
        bytes,
    ))
}
