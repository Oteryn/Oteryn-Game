use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::potion_durations::CAPABILITY_POTION_DURATIONS_V1;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Dispatch {
    if !command.payload.is_empty()
        || !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_POTION_DURATIONS_V1)
    {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    }
    match authority
        .potion_durations_query(*admitted, command.command_id)
        .await
    {
        Some(payload) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload),
        None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
    }
}
