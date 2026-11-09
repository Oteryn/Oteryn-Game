//! Shared receipt discriminators retain both independently owned continuations.
use crate::bestiary_postgres_harness::{self as fixture, TestResult, debug};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use std::sync::Arc;

#[test]
fn engnpc_receipt_cause_union_preserves_existing_guards_and_replay() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .name("engnpc-receipt-causes".into())
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime().map_err(debug)?.block_on(async {
                let started = std::time::Instant::now();
                let h = fixture::Harness::create(admin, "engnpc_causes", true).await?;
                let result = exercise(&h).await;
                h.cleanup().await?;
                result?;
                eprintln!(
                    "POSTGRES_CASE_COMPLETED engnpc_receipt_cause_union_preserves_existing_guards_and_replay migration=1601 duration={:?}",
                    started.elapsed()
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }).map_err(debug)
        })?
        .join()
        .map_err(|_| "receipt causes thread failed")??;
    Ok(())
}

async fn exercise(h: &fixture::Harness) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let quest = "oteryn:quest/engnpc.causes";
    let track = "oteryn:quest-progress/engnpc.causes/accepted";
    let transition = QuestTransition {
        world_change: None,
        key: "oteryn:quest-transition/engnpc.causes/accept".into(),
        quest: quest.into(),
        effects: vec![QuestEffect {
            track: track.into(),
            from: QuestComparison::Eq(0),
            effect: QuestEffectKind::Set(1),
        }],
        completes: false,
        experience: None,
    };
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: track.into(),
                quest: quest.into(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![transition.clone()],
        )
        .map_err(debug)?,
    );
    let request = QuestTransitionRequest {
        transition_key: transition.key,
        cause: QuestCause::Command(CommandRef::new(
            GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(debug)?,
            CommandId::new(1).map_err(debug)?,
        )),
    };
    let outcome = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fixture::fence(1)?,
            request.clone(),
            Arc::clone(&catalogue),
        )
        .await
        .map_err(debug)?;
    let QuestTransitionOutcome::Committed(receipt) = outcome else {
        return Err(format!("expected fixture commit: {outcome:?}").into());
    };
    let replay = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            fixture::fence(1)?,
            request,
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert_eq!(replay, QuestTransitionOutcome::AlreadyCommitted(receipt));
    assert_eq!(h.root_revision().await?, "2");
    assert_eq!(h.count("game_character_quest_receipts").await?, 1);

    // Copy installed CHECKs and unique indexes, not authority/provenance triggers.
    // This tests schema composition without manufacturing trade or timer proofs.
    let mut tx = h.pool.begin().await?;
    sqlx::query("CREATE TEMP TABLE engnpc_receipt_causes (LIKE game_character_quest_receipts INCLUDING CONSTRAINTS INCLUDING INDEXES) ON COMMIT DROP")
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO engnpc_receipt_causes SELECT * FROM game_character_quest_receipts WHERE quest_key=$1")
        .bind(quest).execute(&mut *tx).await?;
    for (cause, ordinal) in [
        ("command", "1"),
        ("use", "1"),
        ("step", "1"),
        ("claim_obligation", "1"),
        ("creature_death", "0"),
        ("server_timer", "0"),
        ("trade_obligation", "1"),
        ("trade_obligation", "18446744073709551615"),
    ] {
        let updated = sqlx::query(
            "UPDATE engnpc_receipt_causes SET cause_kind=$1,cause_ordinal=$2::text::numeric",
        )
        .bind(cause)
        .bind(ordinal)
        .execute(&mut *tx)
        .await?;
        assert_eq!(updated.rows_affected(), 1, "{cause}/{ordinal}");
    }
    for (cause, ordinal) in [
        ("command", "0"),
        ("use", "0"),
        ("step", "0"),
        ("claim_obligation", "0"),
        ("creature_death", "1"),
        ("server_timer", "1"),
        ("trade_obligation", "0"),
        ("trade_obligation", "-1"),
        ("trade_obligation", "18446744073709551616"),
        ("unregistered", "1"),
    ] {
        sqlx::query("SAVEPOINT rejected_cause")
            .execute(&mut *tx)
            .await?;
        let error = sqlx::query(
            "UPDATE engnpc_receipt_causes SET cause_kind=$1,cause_ordinal=$2::text::numeric",
        )
        .bind(cause)
        .bind(ordinal)
        .execute(&mut *tx)
        .await
        .err()
        .ok_or_else(|| format!("invalid receipt cause accepted: {cause}/{ordinal}"))?;
        assert_eq!(
            error
                .as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some("23514")
        );
        sqlx::query("ROLLBACK TO SAVEPOINT rejected_cause")
            .execute(&mut *tx)
            .await?;
        sqlx::query("RELEASE SAVEPOINT rejected_cause")
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("SAVEPOINT duplicate_receipt")
        .execute(&mut *tx)
        .await?;
    let duplicate =
        sqlx::query("INSERT INTO engnpc_receipt_causes SELECT * FROM engnpc_receipt_causes")
            .execute(&mut *tx)
            .await
            .err()
            .ok_or("duplicate receipt accepted")?;
    assert_eq!(
        duplicate
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("23505")
    );
    tx.rollback().await?;
    assert_eq!(h.root_revision().await?, "2");
    assert_eq!(h.count("game_character_quest_receipts").await?, 1);
    let installed: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version=1601 AND success)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert!(
        installed,
        "the canonical migrator must install migration 1601"
    );
    Ok(())
}
