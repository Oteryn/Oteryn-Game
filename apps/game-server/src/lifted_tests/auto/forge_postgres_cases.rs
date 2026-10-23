//! ITEM-FORGE-1 installed storage only. Auto-mounted once, never in dispatch.
//! No migration-owner mutation here claims to be a fenced economic forge.

use crate::item_transfer_postgres_cases as native;
use native::{Harness, STONE, TestResult, configured_admin, debug, runtime};
use oteryn_gameplay_domain::domain::forge::ItemTier;
use oteryn_gameplay_durability::durability::forge::{FORGE_TIER_MIGRATION_VERSION, read_item_tier};
use sqlx::{Connection, PgConnection};

fn run(tag: &str, case: impl AsyncFnOnce(&Harness) -> TestResult) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = Harness::create(admin, tag).await?;
        let result = case(&h).await;
        h.cleanup().await?;
        result
    })
}

async fn mint(h: &Harness) -> TestResult<[u8; 16]> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    h.mint(&authority, STONE, 1).await
}

async fn tier(connection: &mut PgConnection, item: [u8; 16]) -> TestResult<ItemTier> {
    Ok(read_item_tier(connection, item).await.map_err(debug)?.tier)
}

async fn seed(h: &Harness, item: [u8; 16]) -> TestResult {
    sqlx::query(
        "INSERT INTO game_item_tiers(item_instance_id,tier) VALUES(encode($1,'hex')::uuid,1)",
    )
    .bind(item.as_slice())
    .execute(&h.pool)
    .await?;
    Ok(())
}

async fn audit_count(h: &Harness, item: [u8; 16]) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_item_tier_audit WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .fetch_one(&h.pool)
    .await?)
}

fn sqlstate<T>(result: Result<T, sqlx::Error>, code: &str) -> TestResult {
    let error = result.err().ok_or("expected rejected write")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some(code),
        "{error}"
    );
    Ok(())
}

#[test]
fn forge_storage_tier_and_audit_commit_rollback_and_reload() -> TestResult {
    run("forge_storage_atomic", async |h| {
        let installed: bool =
            sqlx::query_scalar("SELECT success FROM _sqlx_migrations WHERE version=$1")
                .bind(FORGE_TIER_MIGRATION_VERSION)
                .fetch_one(&h.pool)
                .await?;
        assert!(installed);
        let item = mint(h).await?;
        let mut connection = PgConnection::connect(&h.database.url).await?;
        assert_eq!(tier(&mut connection, item).await?, ItemTier::ZERO);
        let mut tx = connection.begin().await?;
        sqlx::query("INSERT INTO game_item_tiers VALUES(encode($1,'hex')::uuid,1)")
            .bind(item.as_slice())
            .execute(&mut *tx)
            .await?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_tier_audit WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(item.as_slice()).fetch_one(&mut *tx).await?;
        assert_eq!(count, 1);
        tx.rollback().await?;
        assert_eq!(audit_count(h, item).await?, 0);
        assert_eq!(tier(&mut connection, item).await?, ItemTier::ZERO);

        seed(h, item).await?;
        let mut tx = connection.begin().await?;
        sqlx::query(
            "UPDATE game_item_tiers SET tier=2 WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(item.as_slice())
        .execute(&mut *tx)
        .await?;
        tx.rollback().await?;
        assert_eq!(audit_count(h, item).await?, 1);
        assert_eq!(tier(&mut connection, item).await?.get(), 1);

        // A downstream error rolls back the tier AND the automatically inserted audit.
        let mut tx = connection.begin().await?;
        sqlx::query(
            "UPDATE game_item_tiers SET tier=2 WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(item.as_slice())
        .execute(&mut *tx)
        .await?;
        sqlstate(sqlx::query("SELECT 1/0").execute(&mut *tx).await, "22012")?;
        tx.rollback().await?;
        assert_eq!(audit_count(h, item).await?, 1);
        assert_eq!(tier(&mut connection, item).await?.get(), 1);

        sqlx::query(
            "UPDATE game_item_tiers SET tier=2 WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(item.as_slice())
        .execute(&mut connection)
        .await?;
        connection.close().await?;
        let mut reloaded = PgConnection::connect(&h.database.url).await?;
        assert_eq!(tier(&mut reloaded, item).await?.get(), 2);
        assert_eq!(audit_count(h, item).await?, 2);
        sqlx::query("DELETE FROM game_item_tiers WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(item.as_slice())
            .execute(&mut reloaded)
            .await?;
        assert_eq!(tier(&mut reloaded, item).await?, ItemTier::ZERO);
        let changes: Vec<(Option<i16>, Option<i16>)> = sqlx::query_as(
            "SELECT tier_before,tier_after FROM game_item_tier_audit WHERE item_instance_id=encode($1,'hex')::uuid LIMIT 4")
            .bind(item.as_slice()).fetch_all(&h.pool).await?;
        assert_eq!(changes.len(), 3);
        for change in [(None, Some(1)), (Some(1), Some(2)), (Some(2), None)] {
            assert!(changes.contains(&change));
        }
        reloaded.close().await?;
        Ok(())
    })
}

#[test]
fn forge_storage_runtime_read_only_audit_immutable_and_no_truncate() -> TestResult {
    run("forge_storage_grants", async |h| {
        let item = mint(h).await?;
        seed(h, item).await?;
        for role in [
            "SET LOCAL ROLE oteryn_game_runtime",
            "SET LOCAL ROLE oteryn_game_control",
        ] {
            for table in ["game_item_tiers", "game_item_tier_audit"] {
                let mut tx = h.pool.begin().await?;
                sqlx::query(role).execute(&mut *tx).await?;
                let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                    "SELECT count(*) FROM {table}"
                )))
                .fetch_one(&mut *tx)
                .await?;
                assert_eq!(count, 1);
                tx.rollback().await?;
                for statement in [
                    format!("INSERT INTO {table} DEFAULT VALUES"),
                    format!("DELETE FROM {table}"),
                    format!("TRUNCATE {table}"),
                    format!("UPDATE {table} SET item_instance_id=item_instance_id"),
                ] {
                    let mut tx = h.pool.begin().await?;
                    sqlx::query(role).execute(&mut *tx).await?;
                    sqlstate(
                        sqlx::query(sqlx::AssertSqlSafe(statement))
                            .execute(&mut *tx)
                            .await,
                        "42501",
                    )?;
                    tx.rollback().await?;
                }
            }
        }
        for statement in [
            "DELETE FROM game_item_tier_audit",
            "UPDATE game_item_tier_audit SET tier_after=2",
            "TRUNCATE game_item_tier_audit",
            "TRUNCATE game_item_tiers",
        ] {
            sqlstate(sqlx::query(statement).execute(&h.pool).await, "23514")?;
        }
        assert_eq!(audit_count(h, item).await?, 1);
        Ok(())
    })
}

#[test]
fn forge_storage_concurrent_compare_and_set_has_one_audited_winner() -> TestResult {
    run("forge_storage_concurrent", async |h| {
        let item = mint(h).await?;
        seed(h, item).await?;
        let mut first = h.pool.begin().await?;
        let changed = sqlx::query("UPDATE game_item_tiers SET tier=2 WHERE item_instance_id=encode($1,'hex')::uuid AND tier=1")
            .bind(item.as_slice()).execute(&mut *first).await?;
        assert_eq!(changed.rows_affected(), 1);
        let mut second = PgConnection::connect(&h.database.url).await?;
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut second)
            .await?;
        let waiting = async {
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                loop {
                    let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND wait_event_type='Lock')")
                        .bind(pid).fetch_one(&h.pool).await?;
                    if blocked { break; }
                    tokio::task::yield_now().await;
                }
                first.commit().await
            }).await??;
            Ok::<(), Box<dyn std::error::Error>>(())
        };
        let loser = async {
            Ok::<_, sqlx::Error>(sqlx::query("UPDATE game_item_tiers SET tier=3 WHERE item_instance_id=encode($1,'hex')::uuid AND tier=1")
                .bind(item.as_slice()).execute(&mut second).await?.rows_affected())
        };
        let (wait, lost) = native::join_two(waiting, loser).await;
        wait?;
        assert_eq!(lost?, 0);
        assert_eq!(tier(&mut second, item).await?.get(), 2);
        assert_eq!(audit_count(h, item).await?, 2);
        second.close().await?;
        Ok(())
    })
}
