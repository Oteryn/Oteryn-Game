//! Reuses QUESTL2's atomic-track regression against the canonical durable writer.
use crate::bestiary_postgres_harness::{self as fixture, TestResult, debug};
use crate::domain::CharacterId;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestRefusal, QuestStateCatalogue, QuestTrack,
    QuestTransition,
};
use std::sync::Arc;

#[test]
fn engnpc_sixteen_track_commit_rollback_and_replay() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().name("engnpc-atomic".into()).stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime().map_err(debug)?.block_on(async {
                let started = std::time::Instant::now();
                let harness = fixture::Harness::create(admin, "engnpc_atomic", true).await?;
                let result = exercise(&harness).await;
                harness.cleanup().await?;
                result?;
                eprintln!("POSTGRES_CASE_COMPLETED engnpc_sixteen_track_commit_rollback_and_replay duration={:?}", started.elapsed());
                Ok::<(), Box<dyn std::error::Error>>(())
            }).map_err(debug)
        })?.join().map_err(|_| "engnpc thread failed")??;
    Ok(())
}

async fn exercise(h: &fixture::Harness) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let quest = "oteryn:quest/engnpc.atomic";
    let tracks: Vec<_> = (0..16)
        .map(|i| QuestTrack {
            key: format!("oteryn:quest-progress/engnpc.atomic/{i:02}"),
            quest: quest.into(),
            initial: 0,
            min: 0,
            max: 1,
        })
        .collect();
    let good = QuestTransition {
        world_change: None,
        key: "oteryn:quest-transition/engnpc.atomic/accept".into(),
        quest: quest.into(),
        effects: tracks
            .iter()
            .map(|t| QuestEffect {
                track: t.key.clone(),
                from: QuestComparison::Eq(0),
                effect: QuestEffectKind::Set(1),
            })
            .collect(),
        completes: false,
        experience: None,
    };
    let mut bad = good.clone();
    bad.key = "oteryn:quest-transition/engnpc.atomic/refuse".into();
    bad.effects.last_mut().ok_or("last effect")?.from = QuestComparison::Eq(1);
    let catalog = Arc::new(
        QuestStateCatalogue::new("content-1", tracks, vec![good.clone(), bad.clone()])
            .map_err(debug)?,
    );
    let request = |key: &str, ordinal| -> TestResult<_> {
        Ok(QuestTransitionRequest {
            transition_key: key.into(),
            cause: QuestCause::Command(CommandRef::new(
                GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(debug)?,
                CommandId::new(ordinal).map_err(debug)?,
            )),
        })
    };
    let refused = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fixture::fence(1)?,
            request(&bad.key, 1)?,
            Arc::clone(&catalog),
        )
        .await
        .map_err(debug)?;
    assert_eq!(
        refused,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert_eq!(h.count("game_character_quest_receipts").await?, 0);
    assert_eq!(h.count("game_character_quest_tracks").await?, 0);
    assert_eq!(h.root_revision().await?, "1");
    let command = request(&good.key, 2)?;
    let result = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fixture::fence(1)?,
            command.clone(),
            Arc::clone(&catalog),
        )
        .await
        .map_err(debug)?;
    let QuestTransitionOutcome::Committed(receipt) = result else {
        return Err(format!("expected commit: {result:?}").into());
    };
    assert_eq!(receipt.changes.len(), 16);
    assert!(
        receipt
            .changes
            .iter()
            .all(|c| c.before == 0 && c.after == 1)
    );
    assert_eq!(
        receipt.committed_character_revision.get(),
        receipt.original_character_revision.get() + 1
    );
    assert_eq!(h.count("game_character_quest_receipts").await?, 1);
    let stored: (i64, i64, i64) = sqlx::query_as("SELECT count(*), min(value), max(value) FROM game_character_quest_tracks WHERE quest_key=$1")
        .bind(quest).fetch_one(&h.pool).await?;
    assert_eq!(stored, (16, 1, 1));
    let replay = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fixture::fence(1)?,
            command,
            catalog,
        )
        .await
        .map_err(debug)?;
    assert_eq!(replay, QuestTransitionOutcome::AlreadyCommitted(receipt));
    assert_eq!(h.root_revision().await?, "2");
    let character = CharacterId::from_bytes(fixture::id(fixture::CHARACTER)).map_err(debug)?;
    let copy = h
        .root
        .read_character_quest_state(&authority, character)
        .await
        .map_err(debug)?;
    assert_eq!(copy.tracks().len(), 16);
    // Exercise the installed SQL shape independently of the planner's bound.
    // LIKE copies CHECKs but no mutation triggers or authority rows.
    let mut tx = h.pool.begin().await?;
    sqlx::query("CREATE TEMP TABLE engnpc_receipt_shape (LIKE game_character_quest_receipts INCLUDING CONSTRAINTS) ON COMMIT DROP")
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO engnpc_receipt_shape SELECT * FROM game_character_quest_receipts WHERE quest_key=$1")
        .bind(quest).execute(&mut *tx).await?;
    let error = sqlx::query("INSERT INTO engnpc_receipt_shape SELECT (jsonb_populate_record(NULL::engnpc_receipt_shape, to_jsonb(r) || jsonb_build_object('track_keys', r.track_keys || ARRAY['oteryn:quest-progress/engnpc.atomic/16'], 'values_before', r.values_before || ARRAY[0::bigint], 'values_after', r.values_after || ARRAY[1::bigint]))).* FROM game_character_quest_receipts r WHERE quest_key=$1")
        .bind(quest).execute(&mut *tx).await.err().ok_or("seventeenth SQL effect was accepted")?;
    let database = error
        .as_database_error()
        .ok_or("expected SQL shape refusal")?;
    assert_eq!(database.code().as_deref(), Some("23514"));
    assert_eq!(
        database.constraint(),
        Some("game_character_quest_receipt_effects_shape")
    );
    tx.rollback().await?;
    assert_eq!(h.root_revision().await?, "2");
    Ok(())
}
