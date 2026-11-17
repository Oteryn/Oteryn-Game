//! Fixtures start at M41's owned boundaries; they do not certify the earlier full journeys.
use super::*;
use crate::durability::quest_state::{
    QuestCause, QuestTransitionOutcome, QuestTransitionRequest, npc_program::NpcProgramRequest,
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
    QuestTransition,
    npc_rules::{NpcRuleCatalog, NpcRuleTrigger},
};
use std::sync::Arc;

pub(super) const STAG: &str = "oteryn:quest.the_order_of_the_stag_quest";
pub(super) const KILMARESH: &str = "oteryn:quest.kilmaresh_quest";
pub(super) const FAFNAR: &str = "oteryn:quest-progress/crystalserver/kilmaresh/thirteen/fafnar";
pub(super) const REMAINS: &str = "oteryn:quest-progress/crystalserver/kilmaresh/fourteen/remains";
pub(super) const HARTFORD: &str = "oteryn:npc.guard_captain_hartford";
pub(super) const ACCEPT: &str =
    "oteryn:quest-npc/the_order_of_the_stag/guard_captain_hartford/beaks-accept-advance";
pub(super) const REPORT: &str =
    "oteryn:quest-npc/the_order_of_the_stag/guard_captain_hartford/beaks-report-advance";
pub(super) const STAG_START: &str = "oteryn:quest-transition/m41-fixture/stag-hartford-ready";
pub(super) const STAG_READY: &str = "oteryn:quest-transition/m41-fixture/stag-final-ready";
pub(super) const KILMARESH_READY: &str =
    "oteryn:quest-transition/m41-fixture/kilmaresh-final-ready";

pub(super) fn catalogue() -> TestResult<Arc<QuestStateCatalogue>> {
    let source = NpcRuleCatalog::runtime().map_err(debug)?;
    let mut tracks = Vec::new();
    let mut transitions = Vec::new();
    for quest in [STAG, KILMARESH] {
        let p = source.program(quest).ok_or("M41 actual source programme")?;
        tracks.extend(p.document.tracks.iter().map(|t| QuestTrack {
            key: t.key.clone(),
            quest: t.quest.clone(),
            initial: t.initial,
            min: t.min,
            max: t.max,
        }));
        let digest = p
            .digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        transitions.push(QuestTransition {
            key: format!("oteryn:quest-npc-pin/{digest}"),
            quest: quest.into(),
            effects: p
                .document
                .tracks
                .iter()
                .map(|t| QuestEffect {
                    track: t.key.clone(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Computed,
                })
                .collect(),
            completes: false,
            experience: None,
            world_change: None,
        });
    }
    for (key, quest, track, value) in [
        (STAG_START, STAG, crate::durability::stag_beaks::STAGE, 5),
        (STAG_READY, STAG, crate::durability::stag_beaks::STAGE, 12),
        (KILMARESH_READY, KILMARESH, FAFNAR, 301),
    ] {
        transitions.push(QuestTransition {
            key: key.into(),
            quest: quest.into(),
            effects: vec![QuestEffect {
                track: track.into(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(value),
            }],
            completes: false,
            experience: None,
            world_change: None,
        });
    }
    Ok(Arc::new(
        QuestStateCatalogue::new("content-1", tracks, transitions).map_err(debug)?,
    ))
}

#[allow(
    clippy::too_many_arguments,
    reason = "fixture keeps every NPC request and authority input independently mutable"
)]
pub(super) async fn npc(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    native: &crate::content::native_gameplay::NativeGameplayState,
    catalogue: Arc<QuestStateCatalogue>,
    revision: u64,
    command: u64,
    npc: &str,
    message: &str,
    topic: i32,
    key: &str,
) -> TestResult<crate::durability::quest_state::npc_program::NpcProgramCommit> {
    let permit = owner.spell_lane_permit().await.ok_or("M41 NPC item lane")?;
    // This transport fixture publishes session 50 and lease generation 2.
    // Resolve live authority independently; only the semantic cursor may be retained for replay.
    let session = crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?;
    let mut fence = owner.current_quest_fence(session).await
        .map_err(|_| "M41 current quest fence")?.ok_or("M41 active quest session")?;
    fence.expected_character_revision = crate::domain::CharacterRevision::new(revision).map_err(debug)?;
    owner
        .root
        .commit_character_npc_program(
            owner.character,
            owner.holder,
            &permit,
            fence,
            NpcProgramRequest {
                command: CommandRef::new(
                    fence.game_session_id,
                    CommandId::new(command).map_err(debug)?,
                ),
                npc: npc.into(),
                trigger: NpcRuleTrigger::Say,
                message: message.into(),
                topic,
                transition_key: key.into(),
            },
            catalogue,
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            &owner.premium_coordinator,
        )
        .await
        .map_err(debug)
}

pub(super) async fn ready(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    catalogue: Arc<QuestStateCatalogue>,
    revision: u64,
    command: u64,
    key: &str,
) -> TestResult {
    // This transport fixture publishes session 50 and lease generation 2.
    // Resolve live authority independently; only the semantic cursor may be retained for replay.
    let session = crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?;
    let mut fence = owner.current_quest_fence(session).await
        .map_err(|_| "M41 current quest fence")?.ok_or("M41 active quest session")?;
    fence.expected_character_revision = crate::domain::CharacterRevision::new(revision).map_err(debug)?;
    let outcome = owner
        .root
        .commit_character_quest_transition(
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: key.into(),
                cause: QuestCause::Command(CommandRef::new(
                    fence.game_session_id,
                    CommandId::new(command).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, QuestTransitionOutcome::Committed(_)),
        "{outcome:?}"
    );
    Ok(())
}
