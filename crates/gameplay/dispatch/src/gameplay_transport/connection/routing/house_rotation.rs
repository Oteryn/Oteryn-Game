//! W18 explicit House rotation dispatch, separate from ordinary USE.
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, house_rotation as wire};
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_HOUSE_FURNITURE_ROTATION_V1)
    {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    }
    let Some(request) = wire::Request::decode(command.payload) else {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    };
    let reply = authority
        .house_rotation(*admitted, command.command_id, request)
        .await?;
    Some(Dispatch::Reply(CommandStatus::Accepted, reply.encode()?))
}
