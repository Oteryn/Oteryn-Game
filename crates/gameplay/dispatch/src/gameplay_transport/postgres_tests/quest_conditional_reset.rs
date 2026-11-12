//! Exact-value conditional effects commit on the existing fenced quest receipt owner.
use super::TestResult;
use super::fixture::{Harness, debug, fence};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest/conditional-reset";
const TRACK: &str = "oteryn:quest-progress/conditional-reset/flag";
const TRANSITION: &str = "oteryn:quest-transition/conditional-reset";

#[test]
fn quest_conditional_reset_changes_only_exact_matching_value_and_replays() -> TestResult {
    scenario(1, "conditional-reset-match")
}
#[test]
fn quest_conditional_reset_preserves_absent_initial_flag_and_replays() -> TestResult {
    scenario(-1, "conditional-reset-absent")
}
#[test]
fn quest_conditional_reset_preserves_later_flag_and_replays() -> TestResult {
    scenario(2, "conditional-reset-later")
}

fn scenario(initial: i64, name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run_async(async move |admin| {
        let database_tag = name.replace('-', "_");
        let h = Harness::create(admin, &database_tag, true).await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        println!("POSTGRES_CASE_STARTED {name} server_version={version}");
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let fence = fence(1)?;
        let catalogue = Arc::new(
            QuestStateCatalogue::new(
                "content-1",
                vec![QuestTrack {
                    key: TRACK.into(),
                    quest: QUEST.into(),
                    initial,
                    min: -1,
                    max: 2,
                }],
                vec![QuestTransition {
                    world_change: None,
                    key: TRANSITION.into(),
                    quest: QUEST.into(),
                    completes: false,
                    experience: None,
                    effects: vec![QuestEffect {
                        track: TRACK.into(),
                        from: QuestComparison::Any,
                        effect: QuestEffectKind::SetIf {
                            equals: 1,
                            value: 0,
                        },
                    }],
                }],
            )
            .map_err(debug)?,
        );
        let request = QuestTransitionRequest {
            transition_key: TRANSITION.into(),
            cause: QuestCause::Command(CommandRef::new(
                fence.game_session_id,
                CommandId::new(1).map_err(debug)?,
            )),
        };
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                &h.root,
                &authority,
                &h.node,
                fence,
                request.clone(),
                catalogue.clone(),
            )
            .await
            .map_err(debug)?;
        let QuestTransitionOutcome::Committed(committed) = outcome else {
            return Err(format!("expected conditional commit, got {outcome:?}").into());
        };
        assert_eq!(committed.changes.len(), 1);
        assert_eq!(committed.changes[0].before, initial);
        assert_eq!(
            committed.changes[0].after,
            if initial == 1 { 0 } else { initial }
        );
        assert_eq!(
            slot.commit_quest_transition(&h.root, &authority, &h.node, fence, request, catalogue)
                .await
                .map_err(debug)?,
            QuestTransitionOutcome::AlreadyCommitted(committed)
        );
        let value: i64 =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind(TRACK)
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(value, if initial == 1 { 0 } else { initial });
        let receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(receipts, 1);
        drop(slot);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
