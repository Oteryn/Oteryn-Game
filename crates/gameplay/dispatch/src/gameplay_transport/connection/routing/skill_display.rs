use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Dispatch {
    if !command.payload.is_empty() {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    }
    match authority
        .skill_display_query(*admitted, command.command_id)
        .await
    {
        Some(payload) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload),
        None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
    }
}
