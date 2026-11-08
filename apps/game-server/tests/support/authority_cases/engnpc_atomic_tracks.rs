//! Generic sixteen-track receipt capacity, through the fenced quest writer.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestRefusal, QuestStateCatalogue, QuestTrack,
    QuestTransition,
};
use std::sync::Arc;

#[test]
fn engnpc_sixteen_tracks_commit_refuse_and_replay_atomically() -> fixture::TestResult {
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
                    let h = fixture::Harness::create(admin, "engnpc_atomic_tracks", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGNPC_PG_EXECUTED sixteen_tracks duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "ENGNPC PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let quest = "oteryn:quest.engnpc_atomic_tracks";
    let tracks: Vec<_> = (0..16)
        .map(|i| QuestTrack {
            key: format!("oteryn:quest-progress/engnpc-atomic/{i:02}"),
            quest: quest.into(),
            initial: 0,
            min: 0,
            max: 1,
        })
        .collect();
    let good = QuestTransition {
        key: "oteryn:quest-transition/engnpc-atomic/complete".into(),
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
    bad.key = "oteryn:quest-transition/engnpc-atomic/mismatch".into();
    bad.effects.last_mut().ok_or("last effect")?.from = QuestComparison::Eq(1);
    let catalogue = Arc::new(
        QuestStateCatalogue::new("content-1", tracks, vec![good.clone(), bad.clone()])
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
            transition_key: key.into(),
            cause: QuestCause::Command(CommandRef::new(
                session,
                CommandId::new(ordinal).map_err(fixture::debug)?,
            )),
        })
    };
    let fence = fixture::fence(1)?;
    let before = h
        .root
        .read_character_quest_state(&authority, fence.character_id)
        .await
        .map_err(fixture::debug)?;
    let refused = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fence,
            request(&bad.key, 9000)?,
            catalogue.clone(),
        )
        .await
        .map_err(fixture::debug)?;
    assert_eq!(
        refused,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert_eq!(
        before,
        h.root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(fixture::debug)?
    );
    let command = request(&good.key, 9001)?;
    let committed = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fence,
            command.clone(),
            catalogue.clone(),
        )
        .await
        .map_err(fixture::debug)?;
    let QuestTransitionOutcome::Committed(receipt) = committed else {
        return Err(format!("sixteen-track commit: {committed:?}").into());
    };
    assert_eq!(receipt.changes.len(), 16);
    assert!(
        receipt
            .changes
            .iter()
            .all(|c| c.before == 0 && c.after == 1)
    );
    let persisted: (i64, i32, i64) = sqlx::query_as("SELECT count(*), min(cardinality(track_keys)), min(committed_character_revision-original_character_revision)::bigint FROM game_character_quest_receipts WHERE quest_key=$1").bind(quest).fetch_one(&h.pool).await?;
    assert_eq!(persisted, (1, 16, 1));
    let stored: (i64, i64, i64) = sqlx::query_as("SELECT count(*), min(value), max(value) FROM game_character_quest_tracks WHERE quest_key=$1").bind(quest).fetch_one(&h.pool).await?;
    assert_eq!(stored, (16, 1, 1));
    let replay = h
        .root
        .commit_character_quest_transition(&authority, &h.node, fence, command, catalogue)
        .await
        .map_err(fixture::debug)?;
    assert_eq!(replay, QuestTransitionOutcome::AlreadyCommitted(receipt));
    Ok(())
}
