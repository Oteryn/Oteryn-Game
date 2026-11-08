//! Exact M25 boss predecessors use fenced fixture setup; progress is actual native lethal work.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use std::sync::Arc;
const QUEST: &str = "oteryn:quest.adventures_of_galthen_quest";
const TRACK: &str = "oteryn:quest-progress/chosen-source/adventures_of_galthen_quest/";
#[test]
fn m25_galthen_real_native_deaths_require_predecessors_and_never_complete_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m25-galthen-native-deaths")
}
async fn value(h: &Harness, stage: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
    )
    .bind(format!("{TRACK}{stage}"))
    .fetch_one(&h.pool)
    .await?)
}
async fn predecessor(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    stage: &str,
    ordinal: u64,
) -> TestResult {
    let canonical = owner
        .quest_catalogue
        .as_ref()
        .ok_or("canonical Galthen catalogue")?;
    let key = format!("oteryn:quest-transition/m25-fixture/galthen-{stage}");
    let mut transitions = ["s5", "s8"]
        .into_iter()
        .map(|stage| {
            canonical
                .transition(&format!(
                    "oteryn:quest-transition/chosen-source/adventures_of_galthen_quest/{stage}"
                ))
                .cloned()
                .ok_or("canonical M25 kill transition")
        })
        .collect::<Result<Vec<_>, _>>()?;
    for gate in ["s4", "s7"] {
        transitions.push(QuestTransition {
            world_change: None,
            key: format!("oteryn:quest-transition/m25-fixture/galthen-{gate}"),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: format!("{TRACK}{gate}"),
                from: QuestComparison::Eq(0),
                effect: QuestEffectKind::Set(1),
            }],
            completes: false,
            experience: None,
        });
    }
    let tracks = ["s4", "s5", "s7", "s8"]
        .into_iter()
        .map(|stage| {
            canonical
                .track(&format!("{TRACK}{stage}"))
                .cloned()
                .ok_or("selected authored M25 track")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let catalogue = Arc::new(
        QuestStateCatalogue::new(canonical.content_revision(), tracks, transitions)
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(Arc::clone(&catalogue));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "actual current Quest fence")?
        .ok_or("actual Quest fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: key,
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(ordinal).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("active native policies")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    for (index, (creature, gate, stage)) in [
        ("oteryn:creature.ahau", "s4", "s5"),
        ("oteryn:creature.mitmah_vanguard", "s7", "s8"),
    ]
    .into_iter()
    .enumerate()
    {
        let command = 60000 + u64::try_from(index)? * 10;
        assert_eq!(value(h, gate).await?, 0);
        super::qengine_kill_stories::death(owner, admitted, creature, command).await?;
        super::m25_corpse_chain::assert_native_corpse_chain(&h.pool, creature).await?;
        assert_eq!(
            value(h, stage).await?,
            0,
            "actual death without the exact authored predecessor never counts"
        );
        predecessor(owner, admitted, gate, command + 1).await?;
        assert_eq!(value(h, gate).await?, 1);
        eprintln!(
            "M25_REAL_NATIVE_DEATH_REACHED creature={creature} predecessor={gate} target={stage}"
        );
        super::qengine_kill_stories::death(owner, admitted, creature, command + 2).await?;
        assert_eq!(
            value(h, stage).await?,
            1,
            "real native death progresses its selected authored stage once"
        );
        super::qengine_kill_stories::death(owner, admitted, creature, command + 3).await?;
        assert_eq!(
            value(h, stage).await?,
            1,
            "duplicate projection/replay and another death cannot exceed the stage bound"
        );
        let completed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_quest_states WHERE quest_key=$1 AND completed_character_revision IS NOT NULL)").bind(QUEST).fetch_one(&h.pool).await?;
        assert!(
            !completed,
            "native boss death never completes the quest or performs the final report"
        );
        let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1 AND NOT completes").bind(format!("oteryn:quest-transition/chosen-source/adventures_of_galthen_quest/{stage}")).fetch_one(&h.pool).await?;
        assert_eq!(
            receipts, 1,
            "one actual death owns one non-completing stage receipt"
        );
    }
    Ok(())
}
