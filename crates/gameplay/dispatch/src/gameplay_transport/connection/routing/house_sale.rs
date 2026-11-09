//! T135 shared selected route in both Channel and named-House command loops.
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::gameplay_transport::house_sale::HouseSaleReply;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, house_sale as wire};
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    if command.command_type != wire::COMMAND
        || !admitted
            .continuity
            .selected_capabilities
            .command_selected(wire::COMMAND)
    {
        return Some(Dispatch::Unregistered);
    }
    Some(
        match authority
            .house_sale_command(*admitted, command.command_id, command.payload)
            .await
        {
            HouseSaleReply::Payload(bytes) => Dispatch::Reply(CommandStatus::Accepted, bytes),
            HouseSaleReply::Rejected => Dispatch::Unregistered,
            HouseSaleReply::Unavailable => return None,
        },
    )
}
