//! Isolated PostgreSQL 18.6 database, following the game-server harness's
//! localhost-only test-admin URL and explicit database cleanup conventions.
use super::*;
use crate::store::{IdempotencyStore, NonceStore, PostgresStore};
use sqlx::{
    Connection, Executor, PgConnection,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::str::FromStr;

#[test]
fn postgres_ledger_nonce_and_idempotency() -> Result<(), WireError> {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        if std::env::var_os("REQUIRE_ADMIN_PG").is_some() {
            return Err("PG execution required".into());
        }
        return Ok(());
    }
    let thread = std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(run())
    })?;
    thread.join().map_err(|_| "PG thread failed")?
}
async fn run() -> Result<(), WireError> {
    let url = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
    let options = PgConnectOptions::from_str(&url)?;
    if !url.starts_with("postgresql://oteryn_test_admin:")
        || !url.ends_with("/postgres")
        || !matches!(options.get_host(), "localhost" | "127.0.0.1")
        || options.get_socket().is_some()
        || options.get_username() != "oteryn_test_admin"
        || options.get_database() != Some("postgres")
        || url.contains(['?', '#', '\n', '\r'])
        || ["PGOPTIONS", "PGHOST", "PGSERVICE", "PGSERVICEFILE"]
            .iter()
            .any(|name| std::env::var_os(name).is_some())
    {
        return Err("unsafe PG test route".into());
    }
    let mut admin = PgConnection::connect_with(&options).await?;
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&mut admin)
        .await?;
    if !version.starts_with("18.6 ") && version != "18.6" {
        return Err("PG 18.6 required".into());
    }
    let db = format!("oteryn_game_test_admin_{}", uuid::Uuid::now_v7().simple());
    admin
        .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
            "CREATE DATABASE {db}"
        ))))
        .await?;
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect_with(options.clone().database(&db))
        .await?;
    let result = cases(&pool).await;
    pool.close().await;
    admin
        .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP DATABASE {db}"
        ))))
        .await?;
    result?;
    eprintln!(
        "ADMIN_PG_EXECUTED: PostgreSQL 18.6; isolated DB; real nonce, ledger, idempotency and role checks"
    );
    Ok(())
}
async fn cases(pool: &sqlx::PgPool) -> Result<(), WireError> {
    pool.execute("DO $$ BEGIN IF NOT EXISTS(SELECT FROM pg_roles WHERE rolname='oteryn_game_control') THEN CREATE ROLE oteryn_game_control NOLOGIN; END IF; END $$").await?;
    pool.execute(include_str!(
        "../../../../../apps/game-server/migrations/0956_admin_control_plane.sql"
    ))
    .await?;
    // Re-running the migration preserves data and trigger behavior.
    pool.execute(include_str!(
        "../../../../../apps/game-server/migrations/0956_admin_control_plane.sql"
    ))
    .await?;
    let mut store = PostgresStore { pool: pool.clone() };
    assert!(store.claim_nonce("nonce", NOW).await.map_err(code)?);
    assert!(
        !store
            .claim_nonce("nonce", NOW + 86399)
            .await
            .map_err(code)?
    );
    assert!(
        store
            .claim_nonce("nonce", NOW + 86400)
            .await
            .map_err(code)?
    );
    let mut other = store.clone();
    let (a, b) = tokio::join!(
        store.claim_nonce("race", NOW),
        other.claim_nonce("race", NOW)
    );
    assert_ne!(a.map_err(code)?, b.map_err(code)?);
    let r = request()?;
    let value = response(&r);
    let d = digest(&r.params)?;
    let actual = store.finish(&r, &d, &value, NOW).await.map_err(code)?;
    assert_eq!(actual.game_ledger_id, value.game_ledger_id);
    let mut retry = value.clone();
    retry.game_ledger_id = Some(uuid::Uuid::now_v7().to_string());
    assert_eq!(
        store
            .finish(&r, &d, &retry, NOW)
            .await
            .map_err(code)?
            .game_ledger_id,
        value.game_ledger_id
    );
    assert_eq!(
        store.finish(&r, "changed", &value, NOW).await.err(),
        Some(ErrorCode::AdmIdempotencyConflict)
    );
    let prior = store
        .lookup(&r.admin_operation_id)
        .await
        .map_err(code)?
        .ok_or("no retained result")?;
    assert_eq!(prior.0, d);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM admin_command_ledger")
        .fetch_one(pool)
        .await?;
    assert_eq!(count, 1);
    for sql in [
        "UPDATE admin_command_ledger SET admin_id='other'",
        "DELETE FROM admin_command_ledger",
        "TRUNCATE admin_command_ledger",
    ] {
        assert!(pool.execute(sqlx::AssertSqlSafe(sql)).await.is_err());
    }
    let canonical: Vec<u8> =
        sqlx::query_scalar("SELECT canonical_request FROM admin_command_ledger LIMIT 1")
            .fetch_one(pool)
            .await?;
    assert_eq!(canonical, signed_bytes(&r)?);
    let mut failure = r.clone();
    failure.request_id = "019a5620-0000-7000-8000-000000000003".into();
    failure.admin_operation_id = failure.request_id.clone();
    // Duplicate ledger PK forces a rollback of the idempotency insert too.
    assert!(store.finish(&failure, &d, &value, NOW).await.is_err());
    assert!(
        store
            .lookup(&failure.admin_operation_id)
            .await
            .map_err(code)?
            .is_none()
    );
    let mut conn = pool.acquire().await?;
    sqlx::query("SET ROLE oteryn_game_admin_agent")
        .execute(&mut *conn)
        .await?;
    let can_login: bool =
        sqlx::query_scalar("SELECT rolcanlogin FROM pg_roles WHERE rolname=current_user")
            .fetch_one(&mut *conn)
            .await?;
    assert!(!can_login);
    let privileges:bool=sqlx::query_scalar("SELECT has_table_privilege(current_user,'admin_command_ledger','INSERT') AND NOT has_table_privilege(current_user,'admin_command_ledger','SELECT,UPDATE,DELETE,TRUNCATE') AND NOT has_table_privilege(current_user,'admin_idempotency','UPDATE,DELETE')").fetch_one(&mut *conn).await?;
    assert!(privileges);
    assert!(
        sqlx::query("DELETE FROM admin_command_ledger")
            .execute(&mut *conn)
            .await
            .is_err()
    );
    sqlx::query("RESET ROLE").execute(&mut *conn).await?;
    drop(conn);
    restricted_role_store(pool).await?;
    Ok(())
}
fn code(e: ErrorCode) -> String {
    format!("{e:?}")
}

async fn restricted_role_store(pool: &sqlx::PgPool) -> Result<(), WireError> {
    let agent_pool = PgPoolOptions::new()
        .max_connections(2)
        .after_connect(|conn, _| {
            Box::pin(async move {
                sqlx::query("SET ROLE oteryn_game_admin_agent")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect_with(pool.connect_options().as_ref().clone())
        .await?;
    let mut store = PostgresStore {
        pool: agent_pool.clone(),
    };
    let mut r = request()?;
    r.request_id = "019a5620-0000-7000-8000-000000000004".into();
    r.admin_operation_id = r.request_id.clone();
    let mut value = response(&r);
    value.game_ledger_id = Some(uuid::Uuid::now_v7().to_string());
    let d = digest(&r.params)?;
    assert!(
        store
            .claim_nonce("agent-role-nonce", NOW)
            .await
            .map_err(code)?
    );
    let mut other = store.clone();
    let mut other_value = value.clone();
    other_value.game_ledger_id = Some(uuid::Uuid::now_v7().to_string());
    let (a, b) = tokio::join!(
        store.finish(&r, &d, &value, NOW),
        other.finish(&r, &d, &other_value, NOW)
    );
    assert_eq!(
        a.map_err(code)?.game_ledger_id,
        b.map_err(code)?.game_ledger_id
    );
    assert!(
        store
            .lookup(&r.admin_operation_id)
            .await
            .map_err(code)?
            .is_some()
    );
    let mut rejected = response(&r);
    rejected.game_ledger_id = Some(uuid::Uuid::now_v7().to_string());
    rejected.status = Status::Rejected;
    rejected.error = Some(ErrorDetail {
        code: ErrorCode::AdmInternal,
        message: "test".into(),
        retryable: true,
    });
    store.audit(&r, &d, &rejected, NOW).await.map_err(code)?;
    agent_pool.close().await;
    Ok(())
}
