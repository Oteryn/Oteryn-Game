use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::death_protection::{CAPABILITY_DEATH_PROTECTION_V1, CAPABILITY_DEATH_AND_BLESSINGS_V1, COMMAND_TYPE_DEATH_REPORT_QUERY};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Dispatch {
    let report = command.command_type == COMMAND_TYPE_DEATH_REPORT_QUERY;
    if !command.payload.is_empty()
        || !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_DEATH_PROTECTION_V1)
        || (report && !admitted.continuity.selected_capabilities.contains(CAPABILITY_DEATH_AND_BLESSINGS_V1))
    {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    }
    match authority
        .death_protection_query(*admitted, report)
        .await
    {
        Some(payload) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload),
        None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
    }
}
