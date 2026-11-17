//! Focused table-boundary tests using the production session/table DDL and migration.
use sqlx::{Connection, Executor, PgConnection};

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn fenced_home_town_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(case())
        })?
        .join()
        .map_err(|_| "fenced writer test thread panicked")?
}

async fn case() -> TestResult {
    let mut db = PgConnection::connect(&std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?).await?;
    let mut tx = db.begin().await?;
    // Transaction-local schema: rollback also cleans up after a failed assertion.
    let schema = format!("fenced_writer_{}", std::process::id());
    tx.execute(sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "CREATE SCHEMA {schema}; SET LOCAL search_path = {schema}, pg_catalog; GRANT USAGE ON SCHEMA {schema} TO oteryn_game_runtime"
    )))).await?;
    tx.execute("CREATE TABLE game_character_roots(character_id uuid PRIMARY KEY)")
        .await?;
    let sessions = include_str!(
        "../../../../../../apps/game-server/migrations/0001_admission_reconnect_journal.sql"
    )
    .split_once("CREATE TABLE game_durability_session_replacements")
    .ok_or("session DDL boundary")?
    .0;
    tx.execute(sessions).await?;
    let towns =
        include_str!("../../../../../../apps/game-server/migrations/0534_character_home_town.sql")
            .split_once("CREATE TABLE game_character_home_town_receipts")
            .ok_or("home town DDL boundary")?
            .0;
    tx.execute(towns).await?;
    tx.execute(include_str!(
        "../../../../../../apps/game-server/migrations/0952_fenced_home_town_writer.sql"
    ))
    .await?;
    tx.execute("INSERT INTO game_character_roots VALUES ('00000000-0000-7000-8000-000000000001');
        INSERT INTO game_durability_reconnect_sessions(
          game_session_id,account_id,character_id,world_id,runtime_scope_kind,
          runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,
          original_grace_deadline,predecessor_generation,character_lease_generation,
          scope_ownership_generation,current_generation)
        VALUES ('00000000-0000-7000-8000-000000000002','00000000-0000-7000-8000-000000000003',
          '00000000-0000-7000-8000-000000000001','00000000-0000-7000-8000-000000000004',1,
          '00000000-0000-7000-8000-000000000004','00000000-0000-7000-8000-000000000005',1,100,1,3,4,2)").await?;
    tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
    let write = "SELECT game_character_home_town_write('00000000-0000-7000-8000-000000000001',
        '00000000-0000-7000-8000-000000000002',$1::bigint::numeric,$2::bigint::numeric,$3::bigint::numeric,8)";
    let town: i32 = sqlx::query_scalar(write)
        .bind(2_i64)
        .bind(3_i64)
        .bind(4_i64)
        .fetch_one(&mut *tx)
        .await?;
    assert_eq!(town, 8);
    // Updates as well as inserts pass through the definer.
    sqlx::query(write)
        .bind(2_i64)
        .bind(3_i64)
        .bind(4_i64)
        .execute(&mut *tx)
        .await?;
    for (generation, lease, scope) in [(1_i64, 3_i64, 4_i64), (2, 2, 4), (2, 3, 3)] {
        tx.execute("SAVEPOINT refused").await?;
        let error = sqlx::query(write)
            .bind(generation)
            .bind(lease)
            .bind(scope)
            .execute(&mut *tx)
            .await
            .err()
            .ok_or("stale fence accepted")?;
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("42501")
        );
        tx.execute("ROLLBACK TO SAVEPOINT refused").await?;
    }
    for statement in [
        "INSERT INTO game_character_home_towns VALUES ('00000000-0000-7000-8000-000000000006',9)",
        "UPDATE game_character_home_towns SET town_id=9",
        "DELETE FROM game_character_home_towns",
    ] {
        tx.execute("SAVEPOINT refused").await?;
        let error = tx
            .execute(statement)
            .await
            .err()
            .ok_or("direct DML accepted")?;
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("42501")
        );
        tx.execute("ROLLBACK TO SAVEPOINT refused").await?;
    }
    let stored: i32 = sqlx::query_scalar("SELECT town_id FROM game_character_home_towns")
        .fetch_one(&mut *tx)
        .await?;
    assert_eq!(stored, 8);
    tx.rollback().await?;
    println!(
        "PG_EXECUTED fenced_home_town: current generation accepted; stale connection/lease/scope and runtime INSERT/UPDATE/DELETE refused"
    );
    Ok(())
}
