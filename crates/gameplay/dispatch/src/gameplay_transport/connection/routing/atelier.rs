use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::gameplay_transport::atelier::AtelierWireReply;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    Some(match authority.atelier_command(*admitted, command.command_type, command.payload).await {
        AtelierWireReply::Payload(bytes) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes),
        AtelierWireReply::Rejected => Dispatch::Unregistered,
        AtelierWireReply::Unavailable => return None,
    })
}
