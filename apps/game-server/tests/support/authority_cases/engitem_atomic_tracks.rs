//! ENGITEM: retain the L2 atomic-track regression across the 1349 composition.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestRefusal, QuestStateCatalogue, QuestTrack,
    QuestTransition,
};
use std::sync::Arc;

fn catalogue(count: usize) -> Result<Arc<QuestStateCatalogue>, String> {
    let quest = "oteryn:quest/engitem.atomic";
    let tracks: Vec<_> = (0..count)
        .map(|i| QuestTrack {
            key: format!("oteryn:quest-progress/engitem.atomic/{i:02}"),
            quest: quest.into(),
            initial: 0,
            min: 0,
            max: 1,
        })
        .collect();
    let good = QuestTransition {
        key: "oteryn:quest-transition/engitem.atomic/complete".into(),
        quest: quest.into(),
        effects: tracks
            .iter()
            .map(|t| QuestEffect {
                track: t.key.clone(),
                from: QuestComparison::Eq(0),
                effect: QuestEffectKind::Set(1),
            })
            .collect(),
        completes: true,
        experience: None,
        world_change: None,
    };
    let mut bad = good.clone();
    bad.key = "oteryn:quest-transition/engitem.atomic/refuse".into();
    bad.effects.last_mut().ok_or("last effect")?.from = QuestComparison::Eq(1);
    QuestStateCatalogue::new("content-1", tracks, vec![good, bad])
        .map(Arc::new)
        .map_err(fixture::debug)
}

#[test]
fn engitem_postgres_sixteen_tracks_atomic_refusal_and_replay() -> fixture::TestResult {
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
                    let h = fixture::Harness::create(admin, "engitem_atomic_tracks", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    println!(
                        "ENGITEM_PG_EXECUTED sixteen_tracks duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "ENGITEM PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let migrated: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version=1640 AND success)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert!(migrated, "the focused PG case must execute migration 1640");
    assert!(catalogue(17).is_err(), "seventeenth effect must be refused");
    let catalogue = catalogue(16).map_err(fixture::debug)?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let session = GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?;
    let request = |key: &str, ordinal| -> fixture::TestResult<QuestTransitionRequest> {
        Ok(QuestTransitionRequest {
            transition_key: format!("oteryn:quest-transition/engitem.atomic/{key}"),
            cause: QuestCause::Command(CommandRef::new(
                session,
                CommandId::new(ordinal).map_err(fixture::debug)?,
            )),
        })
    };
    let refused = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fixture::fence(1)?,
            request("refuse", 9000)?,
            Arc::clone(&catalogue),
        )
        .await?;
    assert_eq!(
        refused,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    let counts: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts), (SELECT count(*) FROM game_character_quest_tracks)").fetch_one(&h.pool).await?;
    assert_eq!(counts, (0, 0), "last-track mismatch writes nothing");
    let command = request("complete", 9001)?;
    let committed = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fixture::fence(1)?,
            command.clone(),
            Arc::clone(&catalogue),
        )
        .await?;
    let QuestTransitionOutcome::Committed(receipt) = committed else {
        return Err(format!("sixteen-track commit: {committed:?}").into());
    };
    assert_eq!(receipt.changes.len(), 16);
    let persisted: (i64, i32, i64) = sqlx::query_as("SELECT count(*), min(cardinality(track_keys)), min(committed_character_revision-original_character_revision)::bigint FROM game_character_quest_receipts").fetch_one(&h.pool).await?;
    assert_eq!(persisted, (1, 16, 1));
    let stored: (i64, i64, i64) =
        sqlx::query_as("SELECT count(*), min(value), max(value) FROM game_character_quest_tracks")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(stored, (16, 1, 1));
    let replay = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fixture::fence(1)?,
            command,
            catalogue,
        )
        .await?;
    assert_eq!(replay, QuestTransitionOutcome::AlreadyCommitted(receipt));
    Ok(())
}
