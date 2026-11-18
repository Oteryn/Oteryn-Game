//! The passive donor tick is admitted by the current positioned Channel actor.
use super::*;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};
use oteryn_gameplay_quests::quest::rotten_blood::putrefactory;

#[derive(Debug, Clone)]
pub struct QualifiedPutrefactoryTick {
    fence: CurrentCharacterGameplayFence,
    occurrence: SourceThinkOccurrence,
    proof: [u8; 32],
}

impl QualifiedPutrefactoryTick {
    /// The caller retains the runtime lock until the writer settles the tick.
    pub fn from_current_actor(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        period: u64,
    ) -> Option<Self> {
        runtime.owner_fence().ok()?;
        if period == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
            || runtime.binding().runtime_scope() != fence.runtime_scope
            || runtime.binding().scope_generation() != fence.scope_ownership_generation
        {
            return None;
        }
        let lease = runtime.bound_attacker_lease(actor, fence.game_session_id).ok()?;
        if lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
        {
            return None;
        }
        let snapshot = runtime.read_actor_position(actor).ok()?;
        let at = snapshot.position();
        if !putrefactory::accrues_charge_at(at.x, at.y, at.floor) {
            return None;
        }
        let proof: [u8; 32] = Sha256::new()
            .chain_update(b"oteryn:putrefactory-passive-tick:v1")
            .chain_update(fence.game_session_id.as_bytes())
            .chain_update(actor.placement_identity())
            .chain_update(runtime.map_revision_digest())
            .chain_update(period.to_be_bytes())
            .finalize()
            .into();
        let mut id = *fence.game_session_id.as_bytes();
        id[6..].copy_from_slice(&proof[..10]);
        id[6] = (id[6] & 0x0f) | 0x70;
        id[8] = (id[8] & 0x3f) | 0x80;
        Some(Self {
            fence,
            occurrence: SourceThinkOccurrence::from_bytes(id)?,
            proof,
        })
    }

    pub(super) fn request(&self, fence: &CurrentCharacterGameplayFence) -> Result<QuestTransitionRequest> {
        if self.fence.item_fence() != fence.item_fence() {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        Ok(QuestTransitionRequest {
            transition_key: putrefactory::PASSIVE_TICK.into(),
            cause: QuestCause::SourceThink(self.occurrence),
        })
    }

    pub(super) fn binding(&self, character: CharacterId, request: &QuestTransitionRequest) -> [u8; 33] {
        let mut binding = request_binding(character, request);
        let digest = Sha256::new().chain_update(binding).chain_update(self.proof).finalize();
        binding[1..].copy_from_slice(&digest);
        binding
    }
}

impl DurabilityRoot {
    pub async fn commit_putrefactory_tick(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        input: QualifiedPutrefactoryTick,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome> {
        let request = input.request(&fence)?;
        self.commit_quest_inner::<_, 0>(
            authority, node, fence, request, catalogue,
            None, None, None, None, &grizzly::NoGrizzlyPremium,
            None, None, None, None, None, None, None,
            Some(QuestSourceAction::Putrefactory(input)),
        ).await
    }
}
