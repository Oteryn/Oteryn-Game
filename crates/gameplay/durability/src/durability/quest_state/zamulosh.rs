//! Fenced credit for the qualified lovely-creature movement source.
use super::*;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use oteryn_gameplay_quests::quest::{source_deaths::room_participant, zamulosh};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedLovelyStep {
    player: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    lease_generation: u64,
    digest: [u8; 32],
}

impl QualifiedLovelyStep {
    pub fn capture(
        runtime: &ChannelRuntimeV1,
        monster: ExactActorRef,
        destination: MovementLocalPosition,
        action: u16,
    ) -> Option<(&'static str, Vec<Self>)> {
        runtime.owner_fence().ok()?;
        let (_, kind, room) = zamulosh::PAIRS.iter().find(|(aid, _, _)| *aid == action)?;
        let step = runtime.read_actor_position(monster).ok()?;
        if step.position() != destination
            || step.revision() != step.last_translation_revision()
            || step.revision() == 0
            || !runtime.matches_live_creature_identity(monster, kind.as_bytes())
        {
            return None;
        }
        let digest: [u8; 32] = Sha256::new()
            .chain_update(b"oteryn:zamulosh-lovely-step:v1")
            .chain_update(monster.placement_identity())
            .chain_update(monster.scope_generation().get().to_be_bytes())
            .chain_update(step.revision().to_be_bytes())
            .chain_update(action.to_be_bytes())
            .finalize()
            .into();
        let recipients = runtime
            .positioned_melee_census()
            .ok()?
            .into_iter()
            .filter_map(|(player, at, session)| {
                let session = session?;
                let at = at.position();
                if !room_participant(*room, [at.x, at.y, -i32::from(at.floor)]) {
                    return None;
                }
                let lease = runtime.bound_attacker_lease(player, session).ok()?;
                Some(Self {
                    player,
                    session,
                    character: *lease.character_id().as_bytes(),
                    lease_generation: lease.generation(),
                    digest,
                })
            })
            .collect();
        Some((kind, recipients))
    }

    pub fn session(&self) -> GameSessionId {
        self.session
    }

    pub(super) fn request(
        &self,
        fence: &CurrentCharacterGameplayFence,
    ) -> Result<QuestTransitionRequest> {
        if self.session != fence.game_session_id
            || self.character != *fence.character_id.as_bytes()
            || self.lease_generation != fence.character_lease_generation
            || self.player.runtime_scope() != fence.runtime_scope
            || self.player.scope_generation() != fence.scope_ownership_generation
        {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        Ok(QuestTransitionRequest {
            transition_key: zamulosh::TRANSITION.into(),
            cause: QuestCause::SourceStep(SourceStepOccurrence::from_binding_digest(self.digest)),
        })
    }

    pub(super) fn binding(
        &self,
        character: CharacterId,
        request: &QuestTransitionRequest,
    ) -> [u8; 33] {
        let mut binding = request_binding(character, request);
        let digest = Sha256::new()
            .chain_update(binding)
            .chain_update(self.digest)
            .finalize();
        binding[1..].copy_from_slice(&digest);
        binding
    }
}

impl DurabilityRoot {
    pub async fn commit_zamulosh_lovely_step(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        input: QualifiedLovelyStep,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome> {
        let request = input.request(&fence)?;
        self.commit_quest_inner::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            None,
            None,
            None,
            None,
            &grizzly::NoGrizzlyPremium,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(super::QuestSourceAction::Zamulosh(input)),
        )
        .await
    }
}
