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

/// A habitat timer refers to the exact durable USE that reached seven habitats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HabitatReset {
    command: CommandRef,
    callback: &'static str,
}

impl HabitatReset {
    pub fn from_use(command: CommandRef, callback: &str) -> Option<Self> {
        let habitat = zamulosh::HABITATS.iter().find(|habitat| habitat.key == callback)?;
        Some(Self { command, callback: habitat.key })
    }

    pub fn session(self) -> GameSessionId {
        self.command.game_session_id()
    }

    pub(super) fn request(self, fence: &CurrentCharacterGameplayFence) -> Result<QuestTransitionRequest> {
        if self.session() != fence.game_session_id {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let digest = Sha256::new()
            .chain_update(b"oteryn:zamulosh-habitat-reset:v1")
            .chain_update(self.session().as_bytes())
            .chain_update(self.command.command_id().get().to_be_bytes())
            .chain_update(self.callback.as_bytes())
            .finalize().into();
        Ok(QuestTransitionRequest {
            transition_key: zamulosh::HABITAT_RESET.into(),
            cause: QuestCause::SourceStep(SourceStepOccurrence::from_binding_digest(digest)),
        })
    }

    pub(super) async fn validate_receipt(
        self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, character: CharacterId, now: i64,
    ) -> StoredResult<bool> {
        let valid = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_character_quest_receipts
              WHERE character_id=encode($1,'hex')::uuid AND cause_kind='use'
                AND cause_id=encode($2,'hex')::uuid AND cause_ordinal=$3::text::numeric
                AND transition_key=$4
                AND values_after[array_position(track_keys, $5)] >= 7
                AND committed_at / 1000 <= $6)",
        ).bind(character.as_bytes().as_slice()).bind(self.session().as_bytes().as_slice())
            .bind(self.command.command_id().get().to_string()).bind(self.callback)
            .bind(zamulosh::HABITAT_COUNT)
            .bind(now.saturating_sub(zamulosh::HABITAT_RESET_SECONDS as i64))
            .fetch_one(&mut **tx).await?;
        Ok(valid)
    }
}

impl DurabilityRoot {
    pub async fn commit_zamulosh_habitat_reset(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        input: HabitatReset,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome> {
        let request = input.request(&fence)?;
        self.commit_quest_inner::<_, 0>(
            authority, node, fence, request, catalogue, None, None, None, None,
            &grizzly::NoGrizzlyPremium, None, None, None, None, None, None, None,
            Some(super::QuestSourceAction::HabitatReset(input)),
        ).await
    }
}
