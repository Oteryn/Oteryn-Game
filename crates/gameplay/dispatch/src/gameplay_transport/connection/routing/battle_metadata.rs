use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Dispatch {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(oteryn_protocol_oteryn::attack::CAPABILITY_ATTACK_V1)
    {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    }
    let Some(refs) = oteryn_protocol_oteryn::battle_metadata::decode_query(command.payload) else {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    };
    match authority.battle_metadata_query(*admitted, &refs).await {
        Some(payload) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload),
        None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
    }
}
#[cfg(test)]
#[path = "battle_metadata/tests.rs"]
mod tests;
