//! Current runtime facts consumed alongside the durable session-generation fence.
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeScopeRefV1};
use crate::gameplay_transport::connection::AdmittedSession;

pub(super) fn actor_is_current(runtime: &ChannelRuntimeV1, admitted: AdmittedSession) -> bool {
    let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
        return false;
    };
    let scope = RuntimeScopeRefV1::channel(admitted.world_id, admitted.channel_id);
    fence.game_session_id == admitted.game_session_id
        && fence.connection_generation.get() == admitted.continuity.connection_generation
        && fence.runtime_scope == scope
        && runtime.binding().world_id() == admitted.world_id
        && runtime.binding().channel_id() == Some(admitted.channel_id)
        && runtime
            .player_control_facts(actor, admitted.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
        && runtime
            .bound_attacker_lease(actor, admitted.game_session_id)
            .is_ok_and(|lease| {
                lease.character_id().as_bytes() == fence.character_id.as_bytes()
                    && lease.generation() == fence.character_lease_generation
            })
}

/// Resolve from the live actor census; a browse handle alone is never an invitation proof.
/// The existing party owner independently checks its normal viewport and durable journal.
pub(super) fn party_target(
    runtime: &ChannelRuntimeV1,
    character: [u8; 16],
) -> Option<(ExactActorRef, GameSessionId)> {
    for (actor, _, session) in runtime.positioned_actor_census().ok()? {
        let Some(session) = session else { continue };
        if runtime
            .bound_attacker_lease(actor, session)
            .is_ok_and(|lease| lease.character_id().as_bytes() == &character)
        {
            return Some((actor, session));
        }
    }
    None
}
