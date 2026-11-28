use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::gameplay_transport::house_podium::Request;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, house_podium as wire};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let flagged = admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_HOUSE_PODIUM_FLAGS_V1);
    let full = admitted.continuity.selected_capabilities
        .contains(wire::CAPABILITY_HOUSE_PODIUM_PROJECTION_V2);
    let request = if full && flagged && command.command_type == wire::COMMAND_SET
        && let Some((set, flags, appearance)) = wire::SetRequest::decode_full(command.payload)
    {
        Request::FullSet(set, flags, appearance)
    } else {
        Request::decode_selected(command.command_type, command.payload, flagged)?
    };
    let reply = authority.house_podium(*admitted, request).await?;
    Some(Dispatch::Reply(CommandStatus::Accepted, reply))
}

#[cfg(test)]
#[path = "house_podium/tests.rs"]
mod tests;
