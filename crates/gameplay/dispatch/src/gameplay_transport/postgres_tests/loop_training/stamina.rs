//! Qualify the SQL stamina seam after real wire logout and successor admission.
use super::super::TestResult;
use sqlx::PgPool;

pub(super) async fn qualify(pool: &PgPool) -> TestResult {
    let public: bool = sqlx::query_scalar(
        "SELECT has_function_privilege('public', 'game_training_set_stamina(uuid,uuid,integer)', 'EXECUTE')",
    )
    .fetch_one(pool)
    .await?;
    assert!(!public);
    let (character, occurrence): (String, String) = sqlx::query_as(
        "SELECT character_id::text, last_build_occurrence_id::text FROM game_character_build_state",
    )
    .fetch_one(pool)
    .await?;
    // Once training owns the baseline, even a privileged write must preserve
    // its receipt chain. The pre-training compatibility rule stops here.
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE game_character_build_state SET stamina_minutes=stamina_minutes-1")
        .execute(&mut *tx)
        .await?;
    let error = tx
        .commit()
        .await
        .expect_err("trained stamina remains receipt-owned");
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("P0001")
    );
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let denied = sqlx::query("UPDATE game_character_build_state SET stamina_minutes=0")
        .execute(&mut *tx)
        .await
        .expect_err("runtime cannot assign stamina directly");
    assert_eq!(
        denied.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("42501")
    );
    tx.rollback().await?;

    // A committed receipt cannot be reused to assign stamina in a later transaction.
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    assert!(
        sqlx::query("SELECT game_training_set_stamina($1::uuid,$2::uuid,2520)")
            .bind(&character)
            .bind(&occurrence)
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.rollback().await?;

    // Isolate the function's arithmetic from the surrounding append-only chain:
    // admin fixture edits are rolled back, and only the function runs as runtime.
    // Production settlement above uses its real receipt and immutable clocks.
    for (before, elapsed, expected, requested) in [
        (1000, 779, 1000, Some(1000)),
        (1000, 780, 1001, Some(1001)),
        (2339, 1140, 2341, Some(2341)),
        (2340, 959, 2340, Some(2340)),
        (2340, 960, 2341, Some(2341)),
        (0, 1_814_401, 2520, Some(2520)),
        (1000, 780, 1001, Some(-1)),
        (1000, 780, 1001, Some(2521)),
        (1000, 780, 1001, None),
        (1000, 780, 1001, Some(999)),
        (1000, 780, 1001, Some(1002)),
    ] {
        let mut tx = pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_character_build_state SET stamina_minutes=$1")
            .bind(before)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_character_build_receipts SET stamina_minutes_before=$1, stamina_minutes_after=$2, created_xact_id=pg_current_xact_id() WHERE build_occurrence_id=$3::uuid")
            .bind(before).bind(requested.filter(|v| (0..=2520).contains(v)).unwrap_or(expected))
            .bind(&occurrence).execute(&mut *tx).await?;
        sqlx::query("UPDATE game_character_training_markers m SET committed_at_us=s.training_created_at_us-$1::bigint*1000000, as_of_us=s.training_created_at_us-$1::bigint*1000000 FROM game_durability_reconnect_sessions s JOIN game_character_training_markers current_marker ON current_marker.game_session_id=s.game_session_id WHERE current_marker.build_occurrence_id=$2::uuid AND m.game_session_id=s.training_predecessor_session AND m.cause='logout'")
            .bind(i64::from(elapsed)).bind(&occurrence).execute(&mut *tx).await?;
        sqlx::query("SET LOCAL session_replication_role=origin")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let outcome = sqlx::query("SELECT game_training_set_stamina($1::uuid,$2::uuid,$3)")
            .bind(&character)
            .bind(&occurrence)
            .bind(requested)
            .execute(&mut *tx)
            .await;
        if requested == Some(expected) {
            outcome?;
            let actual: i32 =
                sqlx::query_scalar("SELECT stamina_minutes FROM game_character_build_state")
                    .fetch_one(&mut *tx)
                    .await?;
            assert_eq!(actual, expected, "before={before}, elapsed={elapsed}");
        } else {
            let error = outcome.expect_err("invalid stamina target must be rejected");
            assert_eq!(
                error.as_database_error().and_then(|e| e.code()).as_deref(),
                Some("P0001")
            );
        }
        tx.rollback().await?;
    }
    for (character, occurrence) in [
        (None, Some(occurrence.as_str())),
        (
            Some("00000000-0000-0000-0000-000000000000"),
            Some(occurrence.as_str()),
        ),
        (Some(character.as_str()), None),
        (
            Some(character.as_str()),
            Some("00000000-0000-0000-0000-000000000000"),
        ),
    ] {
        let mut tx = pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let error = sqlx::query("SELECT game_training_set_stamina($1::uuid,$2::uuid,2520)")
            .bind(character)
            .bind(occurrence)
            .execute(&mut *tx)
            .await
            .expect_err("invalid settlement identity must be rejected");
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("P0001")
        );
        tx.rollback().await?;
    }
    Ok(())
}
