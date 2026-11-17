//! Fixture time advances authoritative waits through the ordinary fenced writer.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestRefusal, QuestStateCatalogue, QuestTrack,
    QuestTransition,
};
use std::sync::Arc;

const TIMER: &str = "oteryn:quest-progress/engitem.clock/timer";
const EPOCH: i64 = 1_700_000_000;
const WAIT: i64 = 3600;

#[test]
fn engitem_postgres_quest_clock_preserves_duration_progress_fences_and_replay()
-> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = fixture::Harness::create(admin, "engitem_quest_clock", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    println!(
                        "ENGITEM_PG_EXECUTED quest_clock duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "ENGITEM clock PG thread failed")?
        .map_err(Into::into)
}

async fn snapshot(h: &fixture::Harness) -> fixture::TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('revision',(SELECT character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid),'timer',(SELECT value FROM game_character_quest_tracks WHERE track_key=$2),'receipts',(SELECT count(*) FROM game_character_quest_receipts),'states',(SELECT count(*) FROM game_character_quest_states))")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(TIMER).fetch_one(&h.pool).await?)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let transition = |key: &str, from, completes| QuestTransition {
        key: format!("oteryn:quest-transition/engitem.clock/{key}"),
        quest: "oteryn:quest/engitem.clock".into(),
        effects: vec![QuestEffect {
            track: TIMER.into(),
            from,
            effect: QuestEffectKind::SetNow,
        }],
        completes,
        experience: None,
        world_change: None,
    };
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: TIMER.into(),
                quest: "oteryn:quest/engitem.clock".into(),
                initial: 0,
                min: 0,
                max: i64::MAX,
            }],
            vec![
                transition("start", QuestComparison::Eq(0), false),
                transition("wait", QuestComparison::ElapsedAtLeast(WAIT), true),
            ],
        )
        .map_err(fixture::debug)?,
    );
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let session = GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?;
    let request = |key: &str, ordinal| -> fixture::TestResult<QuestTransitionRequest> {
        Ok(QuestTransitionRequest {
            transition_key: format!("oteryn:quest-transition/engitem.clock/{key}"),
            cause: QuestCause::Command(CommandRef::new(
                session,
                CommandId::new(ordinal).map_err(fixture::debug)?,
            )),
        })
    };
    h.root.set_test_quest_clock(EPOCH)?;
    let start = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fixture::fence(1)?,
            request("start", 9100)?,
            Arc::clone(&catalogue),
        )
        .await?;
    assert!(
        matches!(start, QuestTransitionOutcome::Committed(_)),
        "{start:?}"
    );
    let before = snapshot(h).await?;
    assert_eq!(before["timer"], EPOCH);
    assert_eq!(
        h.root.clone().advance_test_quest_clock(WAIT - 1)?,
        EPOCH + WAIT - 1
    );
    assert_eq!(
        snapshot(h).await?,
        before,
        "advancing time cannot write quest progress"
    );
    let wait = request("wait", 9101)?;
    let refused = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fixture::fence(2)?,
            wait.clone(),
            Arc::clone(&catalogue),
        )
        .await?;
    assert_eq!(
        refused,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert_eq!(
        snapshot(h).await?,
        before,
        "the full authored duration is required"
    );
    h.root.advance_test_quest_clock(1)?;
    // Time control cannot substitute for the current Character revision.
    let stale = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fixture::fence(1)?,
            wait.clone(),
            Arc::clone(&catalogue),
        )
        .await;
    assert!(matches!(
        stale,
        Err(CharacterProgressionError::CharacterRevisionMismatch)
    ));
    assert_eq!(snapshot(h).await?, before);
    let completed = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fixture::fence(2)?,
            wait.clone(),
            Arc::clone(&catalogue),
        )
        .await?;
    let QuestTransitionOutcome::Committed(receipt) = completed else {
        return Err(format!("elapsed wait: {completed:?}").into());
    };
    assert_eq!(receipt.changes[0].after, EPOCH + WAIT);
    let after = snapshot(h).await?;
    assert_eq!(after["receipts"], 2);
    h.root.advance_test_quest_clock(WAIT)?;
    let replay = h
        .root
        .commit_character_quest_transition(&authority, &h.node, fixture::fence(2)?, wait, catalogue)
        .await?;
    assert_eq!(replay, QuestTransitionOutcome::AlreadyCommitted(receipt));
    assert_eq!(snapshot(h).await?, after);
    Ok(())
}
