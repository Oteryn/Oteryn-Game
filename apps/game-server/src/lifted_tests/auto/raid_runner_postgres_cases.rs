//! Migration 0329 qualification only; this does not admit a production raid run.
use crate::bestiary_postgres_harness::{Harness, TestResult, configured_admin, runtime};

#[test]
fn raid_runner_postgres_cursor_receipts_and_privileges() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = Harness::create(admin, "raid_runner", false).await?;
        let result = exercise(&h).await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}

async fn exercise(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_world_raid_schedules VALUES ('00000000-0000-7000-8000-000000000001', 'raid:runner', 1, 1, 0, 2, 60000, 0, NULL, false)")
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_raid_firings VALUES ('00000000-0000-7000-8000-000000000001', 'raid:runner', 1, '00000000-0000-7000-8000-000000000002', 1, 0, 1000, decode(repeat('00',32),'hex'), 1)")
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_raid_channel_runs (firing_id,channel_id,owner_generation,updated_at_ms) VALUES ('00000000-0000-7000-8000-000000000002','00000000-0000-7000-8000-000000000003',1,0)")
        .execute(&mut *tx).await?;
    tx.commit().await?;

    // Rolling back an action transaction loses neither its position nor the
    // ability to retry the same receipt key. No in-memory spawn is simulated.
    for commit in [false, true] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SELECT committed_steps FROM game_world_raid_channel_runs WHERE firing_id='00000000-0000-7000-8000-000000000002' AND channel_id='00000000-0000-7000-8000-000000000003' FOR UPDATE")
            .fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO game_world_raid_step_receipts VALUES ('00000000-0000-7000-8000-000000000002','00000000-0000-7000-8000-000000000003',0,1,10)")
            .execute(&mut *tx).await?;
        sqlx::query("UPDATE game_world_raid_channel_runs SET committed_steps=1,updated_at_ms=10 WHERE firing_id='00000000-0000-7000-8000-000000000002' AND channel_id='00000000-0000-7000-8000-000000000003'")
            .execute(&mut *tx).await?;
        if commit {
            tx.commit().await?;
        } else {
            tx.rollback().await?;
        }
        let position: i32 = sqlx::query_scalar("SELECT committed_steps FROM game_world_raid_channel_runs WHERE firing_id='00000000-0000-7000-8000-000000000002' AND channel_id='00000000-0000-7000-8000-000000000003'")
            .fetch_one(&h.pool).await?;
        assert_eq!(position, i32::from(commit));
    }

    for (statement, code) in [
        (
            "UPDATE game_world_raid_channel_runs SET owner_generation=2",
            "42501",
        ),
        (
            "UPDATE game_world_raid_channel_runs SET committed_steps=0",
            "23514",
        ),
        (
            "UPDATE game_world_raid_channel_runs SET updated_at_ms=9",
            "23514",
        ),
        ("UPDATE game_world_raid_channel_runs SET state=4", "23514"),
        ("DELETE FROM game_world_raid_channel_runs", "42501"),
        (
            "UPDATE game_world_raid_step_receipts SET action_kind=2",
            "42501",
        ),
        ("DELETE FROM game_world_raid_step_receipts", "42501"),
        (
            "INSERT INTO game_world_raid_step_receipts VALUES ('00000000-0000-7000-8000-000000000002','00000000-0000-7000-8000-000000000003',0,1,10)",
            "23505",
        ),
        (
            "INSERT INTO game_world_raid_step_receipts VALUES ('00000000-0000-7000-8000-000000000002','00000000-0000-7000-8000-000000000004',0,1,10)",
            "23503",
        ),
        (
            "INSERT INTO game_world_raid_step_receipts VALUES ('00000000-0000-7000-8000-000000000002','00000000-0000-7000-8000-000000000003',1,4,10)",
            "23514",
        ),
    ] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let error = sqlx::query(sqlx::AssertSqlSafe(statement))
            .execute(&mut *tx)
            .await
            .err()
            .ok_or("invalid runtime mutation must fail")?;
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some(code),
            "{statement}"
        );
        tx.rollback().await?;
    }

    // A new generation loses the old run; it cannot take over and replay it.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_world_raid_channel_runs SET state=3,updated_at_ms=20 WHERE firing_id='00000000-0000-7000-8000-000000000002' AND channel_id='00000000-0000-7000-8000-000000000003'")
        .execute(&mut *tx).await?;
    tx.commit().await?;
    for statement in [
        "UPDATE game_world_raid_channel_runs SET state=1",
        "UPDATE game_world_raid_channel_runs SET state=2",
        "UPDATE game_world_raid_channel_runs SET committed_steps=2",
    ] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let error = sqlx::query(sqlx::AssertSqlSafe(statement))
            .execute(&mut *tx)
            .await
            .err()
            .ok_or("terminal run cannot resume")?;
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        tx.rollback().await?;
    }
    Ok(())
}
