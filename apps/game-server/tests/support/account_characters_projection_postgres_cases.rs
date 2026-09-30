// Shared LCFA-1 cases (migration 0024): the `ListCharactersForAccount` outbox,
// per-account revision and epoch. The cases use SQL only (a root insert is the
// exact row a bootstrap writes), so any PostgreSQL wrapper target can include
// this file. The Rust snapshot/clear/watermark reads over a real bootstrap are
// covered in `character_authority_postgres.rs`.

use sqlx::{Connection, Executor};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn uuid(seed: u8) -> String {
    format!("'{seed:02x}020304-0506-7008-800a-0b0c0d0e0f{seed:02x}'::uuid")
}

struct Database {
    admin_url: String,
    name: String,
    url: String,
}

impl Database {
    async fn create(admin_url: String) -> TestResult<Self> {
        if !admin_url.starts_with("postgresql://oteryn_test_admin:")
            || !admin_url.ends_with("@127.0.0.1:5432/postgres")
        {
            return Err("unsafe PostgreSQL test admin URL".into());
        }
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = format!("lcfa_{suffix}");
        let mut admin = sqlx::PgConnection::connect(&admin_url).await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE DATABASE {name}"
            ))))
            .await?;
        admin.close().await?;
        let prefix = admin_url
            .strip_suffix("/postgres")
            .ok_or("invalid admin URL")?;
        let url = format!("{prefix}/{name}");
        let mut connection = sqlx::PgConnection::connect(&url).await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&mut connection)
            .await?;
        assert_eq!(version, "170006", "canonical target is PostgreSQL 17.6");
        sqlx::migrate!("./migrations").run(&mut connection).await?;
        connection.close().await?;
        Ok(Self {
            admin_url,
            name,
            url,
        })
    }

    async fn cleanup(self) -> TestResult {
        let mut admin = sqlx::PgConnection::connect(&self.admin_url).await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE {} WITH (FORCE)",
                self.name
            ))))
            .await?;
        admin.close().await?;
        Ok(())
    }
}

fn root(character: u8, account: u8, name: &str) -> String {
    format!(
        "INSERT INTO game_character_account_guards VALUES ({account}) ON CONFLICT DO NOTHING; \
         INSERT INTO game_character_roots VALUES ({character},{account},{world},1,1,\
           'profile-1','ruleset-1','content-1','starter-1','{name}');",
        account = uuid(account),
        character = uuid(character),
        world = uuid(90),
    )
}

async fn attempt(pool: &sqlx::PgPool, script: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(script.to_owned()))
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

fn sqlstate(result: &Result<(), sqlx::Error>) -> Option<String> {
    result
        .as_ref()
        .err()
        .and_then(|error| error.as_database_error())
        .and_then(|error| error.code())
        .map(|code| code.into_owned())
}

/// (account seed byte, epoch, revision) of every outbox row, plus each account's revision.
async fn state(pool: &sqlx::PgPool) -> TestResult<(Vec<(i32, i64, i64)>, Vec<(i32, i64)>)> {
    let outbox = sqlx::query_as(
        "SELECT get_byte(uuid_send(account_id), 0), projection_epoch, projection_revision \
           FROM game_character_account_projection_outbox ORDER BY 1, 2, 3",
    )
    .fetch_all(pool)
    .await?;
    let revisions = sqlx::query_as(
        "SELECT get_byte(uuid_send(account_id), 0), projection_revision \
           FROM game_character_account_projections ORDER BY 1",
    )
    .fetch_all(pool)
    .await?;
    Ok((outbox, revisions))
}

async fn epoch(pool: &sqlx::PgPool) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT projection_epoch FROM game_character_account_projection_epoch")
            .fetch_one(pool)
            .await?,
    )
}

/// The adapter's clear statement (`clear_account_characters`).
async fn clear(pool: &sqlx::PgPool, account: u8, epoch: i64, revision: i64) -> TestResult {
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "DELETE FROM game_character_account_projection_outbox WHERE account_id = {} \
           AND projection_epoch <= {epoch} AND projection_revision <= {revision}",
        uuid(account)
    )))
    .execute(pool)
    .await?;
    Ok(())
}

#[test]
fn account_characters_projection_outbox_is_atomic_with_the_character_write() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured");
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let database = Database::create(admin).await?;
            let result = projection_cases(&database).await;
            database.cleanup().await?;
            result
        })
}

async fn projection_cases(database: &Database) -> TestResult {
    let pool = sqlx::PgPool::connect(&database.url).await?;
    assert_eq!(epoch(&pool).await?, 1);

    // A rolled-back Character write leaves no revision and no outbox row.
    let mut tx = pool.begin().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(root(0x11, 0xa0, "Aldric")))
        .execute(&mut *tx)
        .await?;
    tx.rollback().await?;
    assert_eq!(state(&pool).await?, (vec![], vec![]));

    // Committed: the revision advances and the change is queued in the same transaction.
    attempt(&pool, &root(0x11, 0xa0, "Aldric")).await?;
    assert_eq!(state(&pool).await?, (vec![(0xa0, 1, 1)], vec![(0xa0, 1)]));
    // Revision is per account; a second Character of the same account advances it.
    attempt(&pool, &root(0x12, 0xa0, "Bera")).await?;
    attempt(&pool, &root(0x13, 0xa1, "Cato")).await?;
    assert_eq!(
        state(&pool).await?,
        (
            vec![(0xa0, 1, 1), (0xa0, 1, 2), (0xa1, 1, 1)],
            vec![(0xa0, 2), (0xa1, 1)]
        )
    );
    // A refused Character write (taken name, 23505) queues nothing.
    let refused = attempt(&pool, &root(0x14, 0xa1, "al dric")).await;
    assert_eq!(sqlstate(&refused).as_deref(), Some("23505"), "{refused:?}");
    assert_eq!(state(&pool).await?.1, vec![(0xa0, 2), (0xa1, 1)]);
    // The created_at of a queued row is its transaction start, never in the future.
    let future: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_account_projection_outbox \
          WHERE created_at > floor(extract(epoch FROM clock_timestamp()) * 1000)",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(future, 0);

    // Clearing up to the sent revision keeps a later change queued.
    clear(&pool, 0xa0, 1, 1).await?;
    assert_eq!(state(&pool).await?.0, vec![(0xa0, 1, 2), (0xa1, 1, 1)]);

    // Resync without an epoch raise queues every account at its current revision.
    let resync: i64 = sqlx::query_scalar("SELECT game_character_account_projection_resync(false)")
        .fetch_one(&pool)
        .await?;
    assert_eq!(resync, 1);
    assert_eq!(state(&pool).await?.0, vec![(0xa0, 1, 2), (0xa1, 1, 1)]);
    clear(&pool, 0xa0, 1, 2).await?;
    clear(&pool, 0xa1, 1, 1).await?;
    assert_eq!(state(&pool).await?.0, vec![]);

    // An epoch raise exceeds any earlier epoch and the current Unix second, and
    // queues a new-epoch snapshot of every account.
    let raised: i64 = sqlx::query_scalar("SELECT game_character_account_projection_resync(true)")
        .fetch_one(&pool)
        .await?;
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&pool)
        .await?;
    assert!(raised > 1 && raised >= now - 1, "{raised} {now}");
    assert_eq!(epoch(&pool).await?, raised);
    assert_eq!(
        state(&pool).await?.0,
        vec![(0xa0, raised, 2), (0xa1, raised, 1)]
    );
    // An acknowledgement of an older-epoch snapshot of the same revision (in
    // flight across the raise) never clears the new epoch's resync row.
    clear(&pool, 0xa0, 1, 2).await?;
    assert_eq!(state(&pool).await?.0.len(), 2);
    // A change after the raise carries the new epoch and stays queued past an
    // acknowledgement of the earlier revision.
    attempt(&pool, &root(0x15, 0xa1, "Dara")).await?;
    clear(&pool, 0xa1, raised, 1).await?;
    assert_eq!(
        state(&pool).await?.0,
        vec![(0xa0, raised, 2), (0xa1, raised, 2)]
    );
    let raised_again: i64 =
        sqlx::query_scalar("SELECT game_character_account_projection_resync(true)")
            .fetch_one(&pool)
            .await?;
    assert!(raised_again > raised);
    clear(&pool, 0xa0, raised_again, 2).await?;
    clear(&pool, 0xa1, raised_again, 2).await?;
    assert_eq!(state(&pool).await?.0, vec![]);

    // The epoch only rises, a revision only advances by one, and neither table truncates.
    for (case, script, code) in [
        (
            "lower epoch",
            "UPDATE game_character_account_projection_epoch SET projection_epoch = 1",
            "23514",
        ),
        (
            "delete epoch",
            "DELETE FROM game_character_account_projection_epoch",
            "23514",
        ),
        (
            "rewind revision",
            "UPDATE game_character_account_projections SET projection_revision = 1",
            "23514",
        ),
        (
            "truncate revisions",
            "TRUNCATE game_character_account_projections CASCADE",
            "23514",
        ),
        (
            "truncate outbox",
            "TRUNCATE game_character_account_projection_outbox",
            "23514",
        ),
        (
            "resync without choice",
            "SELECT game_character_account_projection_resync(NULL)",
            "22004",
        ),
    ] {
        let result = attempt(&pool, script).await;
        assert_eq!(
            sqlstate(&result).as_deref(),
            Some(code),
            "{case}: {result:?}"
        );
    }

    // PRIV-GUARD-1 and least privilege: the runtime role reads the projection
    // and clears the outbox, but never writes a revision, an epoch or a row,
    // and cannot run the operator resync.
    let runtime = |sql: &str| format!("SET LOCAL ROLE oteryn_game_runtime; {sql}");
    attempt(
        &pool,
        &runtime(
            "SELECT count(*) FROM game_character_account_projection_outbox; \
             SELECT count(*) FROM game_character_account_projections; \
             SELECT projection_epoch FROM game_character_account_projection_epoch; \
             DELETE FROM game_character_account_projection_outbox WHERE false;",
        ),
    )
    .await?;
    for script in [
        format!(
            "INSERT INTO game_character_account_projection_outbox VALUES ({},1,9,0)",
            uuid(0xa0)
        ),
        "UPDATE game_character_account_projections SET projection_revision = projection_revision + 1".into(),
        "UPDATE game_character_account_projection_epoch SET projection_epoch = projection_epoch + 1".into(),
        "SELECT game_character_account_projection_resync(true)".into(),
        format!("SELECT game_character_account_projection_touch({})", uuid(0xa0)),
    ] {
        let result = attempt(&pool, &runtime(&script)).await;
        assert_eq!(sqlstate(&result).as_deref(), Some("42501"), "{script}: {result:?}");
    }
    // The runtime role's own root insert still advances the revision through
    // the definer trigger.
    attempt(&pool, &runtime(&root(0x16, 0xa2, "Eryn"))).await?;
    assert!(state(&pool).await?.1.contains(&(0xa2, 1)));
    pool.close().await;
    Ok(())
}
