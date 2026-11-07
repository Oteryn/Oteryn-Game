use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    Some(
        match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            authority.wheel_allocation_command(*admitted, command.command_type, command.payload),
        )
        .await
        .ok()?
        {
            transport::wheel_allocation::WheelAllocationReply::Payload(bytes) => {
                Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes)
            }
            transport::wheel_allocation::WheelAllocationReply::Rejected => Dispatch::Unregistered,
            transport::wheel_allocation::WheelAllocationReply::Unavailable => return None,
        },
    )
}
