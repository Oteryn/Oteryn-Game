//! Atelier dispatch/model -> JSONB schema qualification. This is deliberately a schema/reload
//! test, not evidence of a production fenced writer or a protocol command (neither exists yet).

#![allow(
    clippy::expect_used,
    reason = "asserting expected SQL refusals in tests"
)]

use crate::bestiary_postgres_harness::{Harness, TestResult, configured_admin, debug, id, runtime};
use crate::gameplay_transport::atelier::{AtelierAction, AtelierState, dispatch_atelier};
use oteryn_gameplay_progression::atelier::{FragmentKind, GemQuality};
use oteryn_gameplay_progression::wheel::WheelDomain;

#[test]
fn atelier_postgres_runtime_grants_grade_json_reload_and_receipt_constraints() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "atelier", true).await?;
        let result = qualify(&h).await;
        h.cleanup().await?;
        result
    })
}

async fn qualify(h: &Harness) -> TestResult {
    let mut state = AtelierState {
        vocation: "elite_knight".to_owned(),
        gold: 100_000_000,
        ..AtelierState::default()
    };
    state.fragments.insert(FragmentKind::Basic, 50);
    for action in [
        AtelierAction::Drop {
            quality: GemQuality::Regular,
        },
        AtelierAction::Reveal {
            id: 1,
            quality: GemQuality::Regular,
            domain: WheelDomain::Green,
            mods: vec![3, 1],
        },
        AtelierAction::Socket { id: 1 },
        AtelierAction::GradeUp {
            kind: FragmentKind::Basic,
            mod_id: 3,
        },
    ] {
        let revision = state.revision;
        dispatch_atelier(&mut state, revision, action).map_err(debug)?;
    }
    let json = serde_json::to_value(&state)?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_atelier_state(character_id,atelier_revision,gold,fragments_basic,fragments_supreme,state_json) VALUES(encode($1,'hex')::uuid,$2,$3,$4,0,$5)")
        .bind(id(41).as_slice()).bind(i64::try_from(state.revision)?)
        .bind(i64::try_from(state.gold)?).bind(i64::from(state.fragments[&FragmentKind::Basic]))
        .bind(&json).execute(&mut *tx).await?;
    // Verify UPDATE permission as well; no production persistence semantics are claimed.
    assert_eq!(sqlx::query("UPDATE game_character_atelier_state SET state_json=$2 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(&json).execute(&mut *tx).await?.rows_affected(), 1);
    sqlx::query("INSERT INTO game_character_atelier_receipts(character_id,occurrence,request_sha256,before_revision,after_revision,outcome_json) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,3,4,$4)")
        .bind(id(41).as_slice()).bind(id(60).as_slice()).bind([7_u8; 32].as_slice())
        .bind(&json).execute(&mut *tx).await?;
    tx.commit().await?;
    // A fresh transaction observes a complete serializable state, including paid grades.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let stored: serde_json::Value = sqlx::query_scalar("SELECT state_json FROM game_character_atelier_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&mut *tx).await?;
    assert_eq!(serde_json::from_value::<AtelierState>(stored)?, state);
    let receipt: serde_json::Value = sqlx::query_scalar("SELECT outcome_json FROM game_character_atelier_receipts WHERE character_id=encode($1,'hex')::uuid AND occurrence=encode($2,'hex')::uuid")
        .bind(id(41).as_slice()).bind(id(60).as_slice()).fetch_one(&mut *tx).await?;
    assert_eq!(receipt, json);
    tx.commit().await?;
    // Receipts stay append-only for the runtime role.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let error = sqlx::query(
        "DELETE FROM game_character_atelier_receipts WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .execute(&mut *tx)
    .await
    .expect_err("runtime must not delete receipts");
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("42501")
    );
    tx.rollback().await?;
    // A negative before revision used to satisfy the successor CHECK (-1 -> 0).
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let error = sqlx::query("INSERT INTO game_character_atelier_receipts(character_id,occurrence,request_sha256,before_revision,after_revision,outcome_json) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,-1,0,$4)")
        .bind(id(41).as_slice()).bind(id(61).as_slice()).bind([7_u8; 32].as_slice())
        .bind(&json).execute(&mut *tx).await.expect_err("negative revision must be refused");
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514")
    );
    tx.rollback().await?;
    Ok(())
}
