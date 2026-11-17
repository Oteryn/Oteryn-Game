use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::gameplay_transport::house_podium::Request;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let flagged = admitted
        .continuity
        .selected_capabilities
        .contains(oteryn_protocol_oteryn::house_podium::CAPABILITY_HOUSE_PODIUM_FLAGS_V1);
    let request = Request::decode_selected(command.command_type, command.payload, flagged)?;
    let reply = authority.house_podium(*admitted, request).await?;
    Some(Dispatch::Reply(CommandStatus::Accepted, reply))
}

#[cfg(test)]
#[path = "house_podium/tests.rs"]
mod tests;
