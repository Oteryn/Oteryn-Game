use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, exiva::Request};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let Some(request) = Request::decode(command.payload) else {
        return Some(Dispatch::Reply(CommandStatus::Rejected, vec![0]));
    };
    let reply = authority
        .exiva_settings(*admitted, command.command_id, request)
        .await?;
    Some(Dispatch::Reply(
        if reply != [0] {
            CommandStatus::Accepted
        } else {
            CommandStatus::Rejected
        },
        reply,
    ))
}
