use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Dispatch {
    match authority
        .server_info_query(*admitted, command.payload)
        .await
    {
        Some(bytes) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, bytes),
        None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
    }
}
