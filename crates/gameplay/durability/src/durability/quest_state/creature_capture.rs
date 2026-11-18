//! A creature capture originates in the Channel's current native target turn.
use super::*;
use crate::foundation::{ChannelRuntimeV1, CharacterLease, ExactActorRef};
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback};

#[derive(Debug, Clone)]
pub struct QualifiedCreatureCapture {
    transition: String,
    definition: [u8; 32],
    occurrence: SourceThinkOccurrence,
    target: ExactActorRef,
    session: GameSessionId,
    lease: CharacterLease,
}

impl QualifiedCreatureCapture {
    pub fn prepare(
        runtime: &ChannelRuntimeV1,
        catalogue: &QuestStateCatalogue,
        callback: &Callback,
        source: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        now: u64,
    ) -> Option<Self> {
        let capture = callback.effects.creature_capture.as_ref()?;
        let transition = callback.creature_capture_key()?;
        let definition = catalogue.definition_hash(&callback.quest)?;
        if runtime.current_live_creature_identity(source).ok()? != capture.creature.as_bytes()
            || runtime.owner_fence().is_err()
            || runtime.current_player_reentry_protection(target, session, now).ok()?
            || catalogue.transition(&transition) != callback.creature_capture_transition().ok()?.as_ref()
        {
            return None;
        }
        let lease = runtime.bound_attacker_lease(target, session).ok()?;
        let origin = runtime.read_actor_position(source).ok()?.position();
        let victim = runtime.read_actor_position(target).ok()?.position();
        if origin.floor != victim.floor
            || origin.x.abs_diff(victim.x).max(origin.y.abs_diff(victim.y)) > u32::from(capture.range)
        {
            return None;
        }
        let hash = Sha256::new()
            .chain_update(b"oteryn:creature-capture:v1")
            .chain_update(source.placement_identity())
            .chain_update(target.placement_identity())
            .chain_update(session.as_bytes())
            .chain_update(definition)
            .chain_update(now.to_be_bytes())
            .finalize();
        let mut bytes: [u8; 16] = hash[..16].try_into().ok()?;
        bytes[6] = (bytes[6] & 0x0f) | 0x70;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Some(Self {
            transition,
            definition,
            occurrence: SourceThinkOccurrence::from_bytes(bytes)?,
            target, session, lease,
        })
    }

    pub fn occurrence(&self) -> [u8; 16] {
        *self.occurrence.as_bytes()
    }

    pub(super) fn current_for(&self, catalogue: &QuestStateCatalogue) -> bool {
        world_callbacks::load_for(catalogue).is_ok_and(|callbacks| {
            callbacks.iter().any(|callback| {
                callback.creature_capture_key().as_deref() == Some(self.transition.as_str())
                    && catalogue.definition_hash(&callback.quest) == Some(self.definition)
            })
        })
    }

    pub(super) fn request(&self, fence: &CurrentCharacterGameplayFence) -> Result<QuestTransitionRequest> {
        if fence.game_session_id != self.session
            || fence.character_id.as_bytes() != self.lease.character_id().as_bytes()
            || fence.character_lease_generation != self.lease.generation()
            || fence.runtime_scope != self.target.runtime_scope()
            || fence.scope_ownership_generation != self.target.scope_generation()
        {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        Ok(QuestTransitionRequest { transition_key: self.transition.clone(), cause: QuestCause::SourceThink(self.occurrence) })
    }

    pub(super) fn binding(&self, character: CharacterId, request: &QuestTransitionRequest) -> [u8; 33] {
        let mut binding = request_binding(character, request);
        let digest = Sha256::new().chain_update(binding)
            .chain_update(self.definition).chain_update(self.target.placement_identity()).finalize();
        binding[1..].copy_from_slice(&digest);
        binding
    }
}

impl DurabilityRoot {
    pub async fn commit_creature_capture(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        capture: QualifiedCreatureCapture,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome> {
        let request = capture.request(&fence)?;
        self.commit_quest_inner::<_, 0>(
            authority, node, fence, request, catalogue, None, None, None, None,
            &grizzly::NoGrizzlyPremium, None, None, None, None, None, None, None,
            Some(super::QuestSourceAction::CreatureCapture(capture)),
        ).await
    }
}
