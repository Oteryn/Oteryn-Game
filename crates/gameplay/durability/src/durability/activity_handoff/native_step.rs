//! Immutable identity of a genuine native translation. Source exit qualification
//! remains with the existing template/map owner; this adds no teleport grant.
use super::*;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};
use oteryn_gameplay_entities::carrier::native_translation_events::CommittedMovementOccurrence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityNativeStepExit {
    movement: CommittedMovementOccurrence,
    session: GameSessionId,
    character: [u8; 16],
    lease: u64,
    proof: serde_json::Value,
    pins: [[u8; 32]; 3],
}
impl ActivityNativeStepExit {
    pub fn from_current_owner(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        movement: CommittedMovementOccurrence,
    ) -> Option<Self> {
        runtime.owner_fence().ok()?;
        let scope = runtime.binding().runtime_scope();
        if !matches!(scope, RuntimeScopeRefV1::Instance { .. })
            || movement.mover() != actor
            || movement.before().position() == movement.after().position()
            || movement.after().revision() <= movement.before().revision()
            || movement.after().revision() != movement.after().last_translation_revision()
            || runtime.read_actor_position(actor).ok()? != movement.after()
            || runtime.committed_movement_occurrence(actor).ok()?? != movement
        {
            return None;
        }
        runtime.player_control_facts(actor, session).ok()?;
        let lease = runtime.bound_attacker_lease(actor, session).ok()?;
        let hex = |bytes: &[u8]| {
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };
        let before = movement.before().position();
        let after = movement.after().position();
        let proof = serde_json::json!({
            "schema":"native-step-exit.v1","movement_binding":hex(&movement.binding()),
            "session":hex(session.as_bytes()),"character":hex(lease.character_id().as_bytes()),
            "lease":lease.generation().to_string(),"scope":hex(&assignment_scope_key(scope)),
            "generation":runtime.binding().scope_generation().get().to_string(),
            "actor":hex(&actor.placement_identity()),"incarnation":actor.actor_local_generation().to_string(),
            "before_revision":movement.before().revision().to_string(),
            "after_revision":movement.after().revision().to_string(),
            "before":[before.x,before.y,before.floor],"after":[after.x,after.y,after.floor],
            "observed_us":movement.observed_us().to_string(),
            "frame":hex(&runtime.content_pin().frame_binding_digest()),
            "map":hex(&runtime.map_revision_digest()),
            "server":hex(&runtime.content_pin().server_artifact_digest())});
        Some(Self {
            movement,
            session,
            character: *lease.character_id().as_bytes(),
            lease: lease.generation(),
            proof,
            pins: [
                runtime.content_pin().frame_binding_digest(),
                runtime.map_revision_digest(),
                runtime.content_pin().server_artifact_digest(),
            ],
        })
    }
    pub fn movement(&self) -> CommittedMovementOccurrence {
        self.movement
    }
    pub fn session(&self) -> GameSessionId {
        self.session
    }
    pub(super) fn matches_request(&self, request: &ActivityReturnRequest) -> bool {
        request.reason == ActivityReturnReason::Exit
            && request.command_id.is_none()
            && self.session == request.source.game_session_id
            && self.character == *request.source.character_id.as_bytes()
            && self.lease == request.source.character_lease_generation
            && self.movement.mover().runtime_scope() == request.source.runtime_scope
            && self.movement.mover().scope_generation() == request.source.scope_ownership_generation
    }
    pub(super) fn bind(&self, hash: &mut Sha256) {
        hash_field(hash, b"native-step-exit.v1");
        hash_field(hash, &self.movement.binding());
        hash_field(hash, &self.movement.observed_us().to_be_bytes());
        for pin in &self.pins {
            hash_field(hash, pin);
        }
    }
    pub(super) fn proof(&self) -> &serde_json::Value {
        &self.proof
    }
}
