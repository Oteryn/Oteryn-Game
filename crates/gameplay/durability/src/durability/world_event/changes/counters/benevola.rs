//! Native Benevola handover joins the existing Quest receipt, carried custody and World owner.
use super::*;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::quest_world_callback::QuestWorldContext;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::CommandRef;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTransition};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const NPC: &str = "oteryn:npc.benevola";
pub const HANDOVER: &str = "oteryn:quest-transition/worldchange/benevola/handover";
pub const WOLF: &str = "oteryn:item.tibia.i12369";
// Crystal 9f5a72c benevola.lua cdc852e9..:129-168 removes ONE captured wolf per yes.
// Wiki 54864@1084984 238b00d7..:5-9 establishes recovery is ten times a wolf kill;
// the shared Counter owner already owns that multiplier, never a personal reward.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenevolaSource {
    pub npc_identity: [u8; 16],
    pub npc_generation: u64,
    pub catalogue_revision: u64,
    pub conversation_revision: u64,
    /// Frozen native source coordinates (positive Tibia z).
    pub position: [u16; 3],
    pub world: QuestWorldContext,
}
#[derive(Debug, Clone)]
pub struct BenevolaHandover {
    pub command: CommandRef,
    pub source: BenevolaSource,
}
impl BenevolaHandover {
    pub(crate) fn valid(&self) -> bool {
        self.source.position == [32596, 31745, 7]
            && self.source.npc_generation > 0
            && self.source.catalogue_revision > 0
            && self.source.conversation_revision > 0
    }
    pub(crate) fn bind(&self, binding: &mut [u8; 33]) -> Result<(), DurabilityError> {
        let source =
            serde_json::to_vec(&self.source).map_err(|_| DurabilityError::InvalidStoredState)?;
        let hash = Sha256::new()
            .chain_update(*binding)
            .chain_update(b"oteryn:benevola:captured-wolf:v1\0")
            .chain_update(source)
            .finalize();
        binding[1..].copy_from_slice(&hash);
        Ok(())
    }
    pub fn catalogue(
        content_revision: &str,
    ) -> Result<Arc<QuestStateCatalogue>, CharacterProgressionError> {
        QuestStateCatalogue::new(
            content_revision,
            Vec::new(),
            vec![QuestTransition {
                key: HANDOVER.into(),
                quest: "oteryn:quest/worldchange/benevola".into(),
                effects: Vec::new(),
                completes: false,
                experience: None,
                world_change: None,
            }],
        )
        .map(Arc::new)
        .map_err(|_| CharacterProgressionError::InvalidInput)
    }
}
impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_benevola_handover(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        permit: &SpellLanePermit,
        handover: BenevolaHandover,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        if !handover.valid()
            || !catalogue.transition(HANDOVER).is_some_and(|transition| {
                transition.quest == "oteryn:quest/worldchange/benevola"
                    && transition.effects.is_empty()
                    && !transition.completes
                    && transition.experience.is_none()
            })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let request = QuestTransitionRequest {
            transition_key: HANDOVER.into(),
            cause: QuestCause::Command(handover.command),
        };
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
            &crate::durability::quest_state::grizzly::NoGrizzlyPremium,
            None,
            None,
            None,
            None,
            Some((permit, handover)),
        )
        .await
    }
}
/// Invoked after the existing Quest writer locks this current Character. A refusal rolls
/// the whole transaction back, including World credit; no separate contribution owner exists.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn consume(
    tx: &mut Transaction<'_, Postgres>,
    permit: &SpellLanePermit,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    handover: &BenevolaHandover,
) -> Result<bool, DurabilityError> {
    if !handover.valid()
        || handover.command.game_session_id() != fence.game_session_id
        || !crate::durability::quest_world_callback::context_is_current(
            tx,
            permit,
            fence,
            &handover.source.world,
        )
        .await?
    {
        return Ok(false);
    }
    let epoch = handover.source.world.reset_epoch;
    let Some(authority) = authorize(
        tx,
        recovery,
        node,
        fence.runtime_scope,
        fence.scope_ownership_generation,
        epoch,
    )
    .await?
    else {
        return Ok(false);
    };
    let Some(day) = super::super::load(tx, fence.runtime_scope.world_id(), epoch).await? else {
        return Ok(false);
    };
    let mut occurrence = Vec::from(fence.character_id.as_bytes().as_slice());
    occurrence.extend_from_slice(handover.command.game_session_id().as_bytes());
    occurrence.extend_from_slice(&handover.command.command_id().get().to_be_bytes());
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let event = CreditEvent {
        source_kind: 3,
        occurrence: occurrence.clone(),
        credits: vec![(CounterKind::Overhunting, Credit::TrappedWolves(1))],
        counter_generations: day.counter_generations,
        occurred_at_ms: u64::try_from(now).map_err(|_| DurabilityError::InvalidStoredState)?,
        character: Some(
            crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        ),
    };
    if !matches!(
        credit(tx, &authority, &event).await?,
        CreditOutcome::Applied { .. }
    ) {
        return Ok(false);
    }
    let proof = serde_json::json!({"native_npc": NPC, "source": handover.source,
        "counter_occurrence": occurrence.iter().map(|b| format!("{b:02x}")).collect::<String>()});
    crate::durability::quest_world_callback::consume_native_npc_one(
        tx,
        permit,
        fence,
        request,
        handover.command,
        WOLF,
        &proof,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worldchg_benevola_closed_source_and_receipt_only_catalogue() -> Result<(), String> {
        let catalogue = BenevolaHandover::catalogue("content-1").map_err(|e| format!("{e:?}"))?;
        let transition = catalogue
            .transition(HANDOVER)
            .ok_or("handover transition")?;
        assert!(transition.effects.is_empty());
        assert!(!transition.completes);
        assert_eq!(transition.experience, None);
        assert!(
            QuestStateCatalogue::new(
                "content-1",
                Vec::new(),
                vec![QuestTransition {
                    key: "oteryn:quest-transition/other/empty".into(),
                    quest: "oteryn:quest/other".into(),
                    effects: Vec::new(),
                    completes: false,
                    experience: None,
                    world_change: None,
                }]
            )
            .is_err()
        );
        Ok(())
    }
}
