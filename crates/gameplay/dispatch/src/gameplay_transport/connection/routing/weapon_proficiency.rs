use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Dispatch {
    let Some(request) = oteryn_protocol_oteryn::weapon_proficiency::decode_request(
        command.command_type,
        command.payload,
    ) else {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    };
    match authority
        .weapon_proficiency_command(*admitted, request)
        .await
    {
        Some(payload) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload),
        None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
    }
}
