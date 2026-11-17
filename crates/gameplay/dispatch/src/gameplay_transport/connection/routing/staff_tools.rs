//! NX-1 signed donor staff tools; shared ingress and session fences remain authoritative.
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
        match transport::staff_tools::dispatch(
            authority,
            *admitted,
            command.command_type,
            command.payload,
        )
        .await
        {
            transport::StaffReply::Result(bytes) => {
                Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes)
            }
            transport::StaffReply::Rejected => {
                Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new())
            }
            transport::StaffReply::Unknown => return None,
        },
    )
}
