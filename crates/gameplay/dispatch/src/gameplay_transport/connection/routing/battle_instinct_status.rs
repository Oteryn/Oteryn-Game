use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::battle_instinct_status::{
    CAPABILITY_BATTLE_INSTINCT_STATUS_V1, decode_query,
};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Dispatch {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_BATTLE_INSTINCT_STATUS_V1)
        || decode_query(command.payload).is_none()
    {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    }
    match authority
        .battle_instinct_status_query(*admitted, command.command_id)
        .await
    {
        Some(payload) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload),
        None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
    }
}

#[cfg(test)]
#[path = "battle_instinct_status/tests.rs"]
mod tests;
