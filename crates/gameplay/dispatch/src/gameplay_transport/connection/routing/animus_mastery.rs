use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::animus_mastery as wire;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Dispatch {
    let Some(page) = wire::decode_query(command.payload) else {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    };
    let include_xp = command.command_type == wire::COMMAND_TYPE_ANIMUS_MASTERY_XP_QUERY;
    match authority
        .animus_mastery_query(*admitted, page, include_xp)
        .await
    {
        Some(payload) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload),
        None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
    }
}
