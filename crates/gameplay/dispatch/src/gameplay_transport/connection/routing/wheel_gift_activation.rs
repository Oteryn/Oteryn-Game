use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::{wheel, world_object::CAPABILITY_GAMEPLAY_OCCURRENCES_V1};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Dispatch {
    if !command.payload.is_empty()
        || !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_GAMEPLAY_OCCURRENCES_V1)
    {
        return Dispatch::Reply(CommandStatus::Rejected, Vec::new());
    }
    match authority
        .wheel_gift_activation(*admitted)
        .await
        .and_then(wheel::GiftActivationReply::encode)
    {
        Some(bytes) => Dispatch::Reply(CommandStatus::Accepted, bytes),
        None => Dispatch::Reply(CommandStatus::Rejected, Vec::new()),
    }
}
