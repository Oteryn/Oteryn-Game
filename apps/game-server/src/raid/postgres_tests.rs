//! Real PostgreSQL qualification of PRIVATE DATA storage only. No native authority is faked.
//! The fixture installs the proposed schema into its own disposable database, never production.
use super::{
    store::{self, Cas},
    test_support::*,
    *,
};
use sqlx::{Connection, Executor};
type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn private_postgres_cas_replay_reload_rollback_and_concurrent_loser() -> TestResult {
    let admin = match std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") {
        Ok(admin) => admin,
        Err(std::env::VarError::NotPresent) => {
            eprintln!(
                "PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured"
            );
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    if !admin.starts_with("postgresql://oteryn_test_admin:")
        || !admin.ends_with("@127.0.0.1:5432/postgres")
    {
        return Err("unsafe PostgreSQL test-admin route".into());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let (name, url, mut connection) = runtime.block_on(async {
        let name = format!(
            "raid_data_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let mut connection = sqlx::PgConnection::connect(&admin).await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&mut connection)
            .await?;
        if version != "170006" {
            return Err(format!("canonical PG17.6 required, got {version}").into());
        }
        connection
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE DATABASE {name}"
            ))))
            .await?;
        let url = format!(
            "{}/{}",
            admin.strip_suffix("/postgres").ok_or("invalid admin URL")?,
            name
        );
        Ok::<_, Box<dyn std::error::Error>>((name, url, connection))
    })?;
    // A negative assertion or controlled mutant must still remove its exact fixture database.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.block_on(qualify(&url))
    }));
    runtime.block_on(async {
        connection
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE {name} WITH (FORCE)"
            ))))
            .await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    })?;
    match result {
        Ok(result) => result,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

async fn qualify(url: &str) -> TestResult {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(url)
        .await?;
    let outcome = qualify_pool(&pool, url).await;
    pool.close().await;
    outcome
}

async fn qualify_pool(pool: &sqlx::PgPool, url: &str) -> TestResult {
    sqlx::raw_sql(store::PRIVATE_SCHEMA).execute(pool).await?;
    eprintln!(
        "canonical PostgreSQL17.6; private uninstalled raid schema; data CAS only, no native authority qualification"
    );
    let initialize = initial(false);
    let mut tx = pool.begin().await?;
    assert_eq!(store::apply(&mut tx, &initialize).await?, Cas::Applied);
    tx.commit().await?;
    let mut tx = pool.begin().await?;
    assert_eq!(store::apply(&mut tx, &initialize).await?, Cas::Replayed);
    tx.commit().await?;
    let mut tx = pool.begin().await?;
    let loaded = store::load(&mut tx, &key())
        .await?
        .ok_or("missing schedule")?;
    assert_eq!(loaded, initialize.after);
    tx.rollback().await?;

    let fire = loaded.check(&facts(60_000))?.ok_or("due check")?;
    let mut tx = pool.begin().await?;
    assert_eq!(store::apply(&mut tx, &fire).await?, Cas::Applied);
    tx.rollback().await?;
    let mut tx = pool.begin().await?;
    assert_eq!(store::load(&mut tx, &key()).await?, Some(loaded.clone()));
    tx.rollback().await?;

    // Distinct checks share the exact same expected snapshot. Both transactions overlap;
    // the first owns the named lock before the second begins. Exactly one proposal applies.
    let mut different = facts(60_001);
    different.proposed_firing_id = id(11);
    let loser = loaded.check(&different)?.ok_or("other check")?;
    let mut first = pool.begin().await?;
    assert_eq!(store::apply(&mut first, &fire).await?, Cas::Applied);
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let url = url.to_owned();
    let worker = std::thread::spawn(move || -> std::result::Result<Cas, String> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?
            .block_on(async {
                let pool = sqlx::PgPool::connect(&url)
                    .await
                    .map_err(|e| e.to_string())?;
                let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
                started_tx.send(()).map_err(|e| e.to_string())?;
                let result = store::apply(&mut tx, &loser)
                    .await
                    .map_err(|e| e.to_string())?;
                tx.commit().await.map_err(|e| e.to_string())?;
                pool.close().await;
                Ok(result)
            })
    });
    started_rx.recv_timeout(std::time::Duration::from_secs(10))?;
    first.commit().await?;
    assert_eq!(
        worker
            .join()
            .map_err(|_| "concurrent fixture thread panicked")??,
        Cas::CompareFailed
    );
    let mut tx = pool.begin().await?;
    assert_eq!(store::apply(&mut tx, &fire).await?, Cas::Replayed);
    let current = store::load(&mut tx, &key()).await?.ok_or("firing")?;
    assert_eq!(current, fire.after);
    tx.commit().await?;
    let claim = current
        .claim(id(10), channel(2), generation(5))?
        .ok_or("claim metadata")?;
    let mut tx = pool.begin().await?;
    assert_eq!(store::apply(&mut tx, &claim).await?, Cas::Applied);
    tx.commit().await?;
    let recovery = claim
        .after
        .claim(id(10), channel(2), generation(6))?
        .ok_or("lost proposal")?;
    let mut tx = pool.begin().await?;
    assert_eq!(store::apply(&mut tx, &recovery).await?, Cas::Applied);
    tx.commit().await?;
    let closed = recovery.after.close(90_000)?.ok_or("deadline")?;
    let mut tx = pool.begin().await?;
    assert_eq!(store::apply(&mut tx, &closed).await?, Cas::Applied);
    tx.commit().await?;

    // Fresh pool connection reload: missing target became LOST; old-generation run stays LOST.
    let mut tx = pool.begin().await?;
    let durable = store::load(&mut tx, &key()).await?.ok_or("reload")?;
    assert_eq!(durable, closed.after);
    assert_eq!(store::apply(&mut tx, &claim).await?, Cas::CompareFailed);
    tx.rollback().await?;
    let mut other = key();
    other.world = WorldId::decode(&id(99))?;
    let mut tx = pool.begin().await?;
    assert!(store::load(&mut tx, &other).await?.is_none());
    tx.rollback().await?;

    // A new firing retains the previous firing and runs rather than erasing history.
    let mut next_facts = facts(120_000);
    next_facts.proposed_firing_id = id(12);
    let next = durable.check(&next_facts)?.ok_or("next check")?;
    let mut tx = pool.begin().await?;
    assert_eq!(store::apply(&mut tx, &next).await?, Cas::Applied);
    tx.commit().await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_firings")
        .fetch_one(pool)
        .await?;
    assert_eq!(count, 2);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_channel_runs")
        .fetch_one(pool)
        .await?;
    assert_eq!(count, 2);

    // Corruption cases change exactly one stored invariant each and roll back their mutation.
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE game_world_raid_schedules SET revision=revision+1")
        .execute(&mut *tx)
        .await?;
    assert!(matches!(
        store::load(&mut tx, &key()).await,
        Err(store::StoreError::Invalid(RaidError::InvalidStoredState))
    ));
    tx.rollback().await?;
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO game_world_raid_channel_runs VALUES(encode($1,'hex')::uuid,$2,1,$3,$4)",
    )
    .bind(key().world.as_bytes().as_slice())
    .bind(key().raid)
    .bind(channel(99).as_bytes().as_slice())
    .bind(br#"{"Lost":null}"#.as_slice())
    .execute(&mut *tx)
    .await?;
    assert!(matches!(
        store::load(&mut tx, &key()).await,
        Err(store::StoreError::Invalid(RaidError::InvalidStoredState))
    ));
    tx.rollback().await?;
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE game_world_raid_firings SET firing_id=$1 WHERE ordinal=1")
        .bind(id(44).as_slice())
        .execute(&mut *tx)
        .await?;
    assert!(matches!(
        store::load(&mut tx, &key()).await,
        Err(store::StoreError::Invalid(RaidError::InvalidStoredState))
    ));
    tx.rollback().await?;
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO game_world_raid_channel_runs VALUES(encode($1,'hex')::uuid,$2,1,$3,$4)",
    )
    .bind(key().world.as_bytes().as_slice())
    .bind(key().raid)
    .bind(channel(2).as_bytes().as_slice())
    .bind(br#"{"Claimed":0}"#.as_slice())
    .execute(&mut *tx)
    .await?;
    assert!(matches!(
        store::load(&mut tx, &key()).await,
        Err(store::StoreError::Invalid(RaidError::InvalidStoredState))
    ));
    tx.rollback().await?;
    let mut tx = pool.begin().await?;
    assert_eq!(
        store::load(&mut tx, &key()).await?,
        Some(next.after.clone())
    );
    tx.rollback().await?;
    // Actual ordinal9→10 boundary: MAX is a separate numeric oracle. The production loader
    // must order the numeric table column, not its same-named ::text output expression.
    let mut durable = next.after;
    for ordinal in 2..=11 {
        let deadline = durable.latest.as_ref().ok_or("missing firing")?.close_at_ms;
        let close = durable.close(deadline)?.ok_or("close before next firing")?;
        let mut tx = pool.begin().await?;
        assert_eq!(store::apply(&mut tx, &close).await?, Cas::Applied);
        tx.commit().await?;
        let mut input = facts(close.after.next_check_at_ms());
        input.proposed_firing_id = id(20 + ordinal);
        let fire = close.after.check(&input)?.ok_or("next due firing")?;
        let mut tx = pool.begin().await?;
        assert_eq!(store::apply(&mut tx, &fire).await?, Cas::Applied);
        tx.commit().await?;
        let max: String =
            sqlx::query_scalar("SELECT max(ordinal)::text FROM game_world_raid_firings")
                .fetch_one(pool)
                .await?;
        assert_eq!(max, ordinal.to_string());
        eprintln!(
            "real PG numeric MAX firing ordinal={max}; checking actual loader at 9/10/11 boundary"
        );
        let mut tx = pool.begin().await?;
        let reloaded = store::load(&mut tx, &key())
            .await?
            .ok_or("latest firing missing")?;
        assert_eq!(reloaded, fire.after);
        tx.rollback().await?;
        durable = reloaded;
    }
    Ok(())
}
