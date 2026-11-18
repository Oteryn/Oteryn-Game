use super::*;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};

/// Binds an authorized map USE to the current Instance actor and Channel landing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityNativeUseExit {
    actor: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    lease: u64,
    command_id: u64,
    pub(super) destination: LastPositionCell,
    position_revision: u64,
}

impl ActivityNativeUseExit {
    pub fn from_current_owner(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        destination: LastPositionCell,
    ) -> Option<Self> {
        runtime.owner_fence().ok()?;
        if command_id == 0
            || !matches!(runtime.binding().runtime_scope(), RuntimeScopeRefV1::Instance { .. })
            || runtime.player_control_facts(actor, session).ok()?.control_loss.is_some()
        {
            return None;
        }
        let lease = runtime.bound_attacker_lease(actor, session).ok()?;
        Some(Self {
            actor,
            session,
            character: *lease.character_id().as_bytes(),
            lease: lease.generation(),
            command_id,
            destination,
            position_revision: runtime.read_actor_position(actor).ok()?.revision(),
        })
    }

    pub(super) fn matches_request(&self, request: &ActivityReturnRequest) -> bool {
        request.reason == ActivityReturnReason::Exit
            && request.command_id == Some(self.command_id)
            && request.destination_transport.is_some()
            && self.session == request.source.game_session_id
            && self.character == *request.source.character_id.as_bytes()
            && self.lease == request.source.character_lease_generation
            && self.actor.runtime_scope() == request.source.runtime_scope
            && self.actor.scope_generation() == request.source.scope_ownership_generation
    }

    pub(super) fn bind(&self, hash: &mut Sha256) {
        hash_field(hash, b"native-use-exit.v1");
        hash_field(hash, &self.command_id.to_be_bytes());
        hash_field(hash, &self.actor.placement_identity());
        hash_field(hash, &self.actor.actor_local_generation().to_be_bytes());
        hash_field(hash, &self.position_revision.to_be_bytes());
        hash_field(hash, &self.destination.x.to_be_bytes());
        hash_field(hash, &self.destination.y.to_be_bytes());
        hash_field(hash, &self.destination.floor.to_be_bytes());
    }
}
