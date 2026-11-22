use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::cyclopedia_character::kill_credits::{
    self, COMMAND_TYPE_PLAYER_KILL_CREDITS,
};
use oteryn_protocol_oteryn::death_protection::{
    CAPABILITY_COMBAT_OCCURRENCES_V1, COMMAND_TYPE_DEATH_OCCURRENCE,
};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Dispatch {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_COMBAT_OCCURRENCES_V1)
    {
        return Dispatch::Reply(CommandStatus::Rejected, Vec::new());
    }
    let reply = match command.command_type {
        COMMAND_TYPE_DEATH_OCCURRENCE if command.payload.is_empty() => {
            authority.death_occurrence(*admitted).await
        }
        COMMAND_TYPE_PLAYER_KILL_CREDITS => match kill_credits::decode_request(command.payload) {
            Some(page) => authority.player_kill_credits(*admitted, page).await,
            None => None,
        },
        _ => None,
    };
    match reply {
        Some(bytes) => Dispatch::Reply(CommandStatus::Accepted, bytes),
        None => Dispatch::Reply(CommandStatus::Rejected, Vec::new()),
    }
}
