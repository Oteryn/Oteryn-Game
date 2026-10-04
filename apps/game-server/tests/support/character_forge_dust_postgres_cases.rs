// FORGE-1a forge dust balance and ledger (migration 0059). Any wrapper that provides the same
// path-loaded crate root as `character_forge_dust_postgres.rs` can include this file.
//
// No production source writes dust yet, so each case is the caller's transaction: the runtime
// role locks the Character root (rule 4) and calls the writer, then commits or rolls back. The
// Character root is seeded by the migration owner with the guards off, as other cases do.

use crate::domain::forge_dust::{DustLimit, ForgeDustBalance};
use crate::domain::{CharacterId, CharacterRevision};
use crate::durability::character_forge_dust::{
    ForgeDustEntryKind, ForgeDustGainCause, ForgeDustOutcome, ForgeDustSpendCause,
    ForgeDustWriteError, ForgeDustWriteIds, gain_forge_dust_in_transaction, read_forge_dust,
    spend_forge_dust_in_transaction,
};
use crate::durability::character_forge_dust_audit::{
    ForgeDustAuditError, ForgeDustLedgerAudit, audit_forge_dust_ledger,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::{
    ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    WorldId,
};
use sqlx::postgres::PgConnection;
use sqlx::{Connection, Executor, PgPool};
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CHARACTER: u8 = 41;
const ACCOUNT: u8 = 44;
const WORLD: u8 = 42;
const CHANNEL: u8 = 43;
const OTHER_WORLD: u8 = 46;

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

fn debug(error: impl std::fmt::Debug) -> Box<dyn std::error::Error> {
    format!("{error:?}").into()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn uuid(seed: u8) -> String {
    format!("'{}'::uuid", hex(&id(seed)))
}

fn configured_admin() -> Option<String> {
    match std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") {
        Ok(value) => Some(value),
        Err(_) => {
            eprintln!(
                "PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured"
            );
            None
        }
    }
}

fn run<F>(body: F) -> TestResult
where
    F: AsyncFnOnce(String) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(body(admin))
}

struct Harness {
    admin_url: String,
    name: String,
    /// The migration owner, for fixtures and read-back only.
    pool: PgPool,
    /// A login in the runtime group (0006): every dust write runs as it.
    runtime: PgPool,
}

impl Harness {
    async fn create(admin_url: String, tag: &str) -> TestResult<Self> {
        if !admin_url.starts_with("postgresql://oteryn_test_admin:")
            || !admin_url.ends_with("@127.0.0.1:5432/postgres")
        {
            return Err("unsafe PostgreSQL test admin URL".into());
        }
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = format!("dust_{tag}_{suffix}");
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
        let pool = PgPool::connect(&url).await?;

        let role = format!("dust_runtime_{tag}_{suffix}");
        let password = format!("{role}-secret");
        for statement in [
            format!("CREATE ROLE {role} LOGIN PASSWORD '{password}' IN ROLE oteryn_game_runtime"),
            format!("GRANT CONNECT ON DATABASE {name} TO {role}"),
        ] {
            sqlx::query(sqlx::AssertSqlSafe(statement))
                .execute(&pool)
                .await?;
        }
        let (_, address) = url.split_once('@').ok_or("no authority separator")?;
        let runtime = PgPool::connect(&format!("postgresql://{role}:{password}@{address}")).await?;

        let harness = Self {
            admin_url,
            name,
            pool,
            runtime,
        };
        harness
            .seed(&format!(
                "INSERT INTO game_character_roots VALUES ({character}, {account}, {world}, 1, 1, \
                   'profile-1', 'ruleset-1', 'content-1', 'starter-1', 'Dust Hero');",
                character = uuid(CHARACTER),
                account = uuid(ACCOUNT),
                world = uuid(WORLD),
            ))
            .await?;
        Ok(harness)
    }

    /// These cases spend through the 0059 writer alone. 0060 pairs every `proficiency` SPEND with
    /// its modification line at commit (covered by the proficiency modification cases), so a case
    /// that commits a bare spend switches that one pairing trigger off in its own database.
    async fn standalone_spends(&self) -> TestResult {
        sqlx::query(
            "ALTER TABLE game_character_forge_dust_entries \
             DISABLE TRIGGER game_character_forge_dust_entry_proficiency_line",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Fixture statements as the migration owner with every trigger off.
    async fn seed(&self, script: &str) -> TestResult {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(script.to_owned()))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn entries(&self) -> TestResult<i64> {
        Ok(
            sqlx::query_scalar("SELECT count(*) FROM game_character_forge_dust_entries")
                .fetch_one(&self.pool)
                .await?,
        )
    }

    async fn balance(&self) -> TestResult<ForgeDustBalance> {
        let mut connection = self.pool.acquire().await?;
        read_forge_dust(&mut connection, character()?)
            .await
            .map_err(debug)
    }

    async fn audit(&self) -> TestResult<ForgeDustLedgerAudit> {
        let mut connection = self.pool.acquire().await?;
        audit_forge_dust_ledger(&mut connection, character()?, 1_000)
            .await
            .map_err(debug)
    }

    async fn cleanup(self) -> TestResult {
        self.runtime.close().await;
        self.pool.close().await;
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

fn character() -> TestResult<CharacterId> {
    CharacterId::from_bytes(id(CHARACTER)).map_err(debug)
}

fn fence_in(world: u8) -> TestResult<CurrentCharacterGameplayFence> {
    Ok(CurrentCharacterGameplayFence {
        character_id: character()?,
        game_session_id: GameSessionId::decode(&id(45)).map_err(debug)?,
        connection_generation: ConnectionGeneration::new(1).map_err(debug)?,
        character_lease_generation: 1,
        runtime_scope: RuntimeScopeRefV1::channel(
            WorldId::decode(&id(world)).map_err(debug)?,
            ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
        ),
        scope_ownership_generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
        expected_character_revision: CharacterRevision::new(1).map_err(debug)?,
    })
}

/// Occurrence `occurrence` writes entry `occurrence + 100` in TransactionId `occurrence + 130`.
fn ids(occurrence: u8) -> ForgeDustWriteIds {
    ForgeDustWriteIds {
        entry_id: id(occurrence + 100),
        transaction_id: id(occurrence + 130),
    }
}

#[derive(Debug, Clone, Copy)]
enum Op {
    Gain(u8, u32),
    Spend(u8, u32),
}

async fn lock_root(connection: &mut PgConnection) -> TestResult {
    sqlx::query(
        "SELECT 1 FROM game_character_roots WHERE character_id = encode($1,'hex')::uuid \
         FOR UPDATE",
    )
    .bind(id(CHARACTER).as_slice())
    .execute(&mut *connection)
    .await?;
    Ok(())
}

async fn write(
    connection: &mut PgConnection,
    world: u8,
    op: Op,
) -> TestResult<Result<ForgeDustOutcome, ForgeDustWriteError>> {
    let fence = fence_in(world)?;
    Ok(match op {
        Op::Gain(occurrence, amount) => {
            let cause = ForgeDustGainCause::CreatureKill {
                occurrence: id(occurrence),
            };
            gain_forge_dust_in_transaction(connection, &fence, cause, amount, ids(occurrence)).await
        }
        Op::Spend(occurrence, amount) => {
            let cause = ForgeDustSpendCause::Proficiency {
                occurrence: id(occurrence),
            };
            spend_forge_dust_in_transaction(connection, &fence, cause, amount, ids(occurrence))
                .await
        }
    })
}

/// One caller transaction as the runtime role: the root lock (rule 4), the write, then commit,
/// or rollback on a refusal.
async fn commit(
    harness: &Harness,
    op: Op,
) -> TestResult<Result<ForgeDustOutcome, ForgeDustWriteError>> {
    let mut tx = harness.runtime.begin().await?;
    lock_root(&mut tx).await?;
    let outcome = write(&mut tx, WORLD, op).await?;
    if outcome.is_ok() {
        tx.commit().await?;
    } else {
        tx.rollback().await?;
    }
    Ok(outcome)
}

fn written(outcome: Result<ForgeDustOutcome, ForgeDustWriteError>) -> TestResult<(u16, u32)> {
    match outcome {
        Ok(ForgeDustOutcome::Written(entry)) => Ok((entry.after.balance(), entry.lost)),
        other => Err(format!("expected a written entry, got {other:?}").into()),
    }
}

fn balance(value: u16) -> TestResult<ForgeDustBalance> {
    ForgeDustBalance::new(value, DustLimit::INITIAL).map_err(debug)
}

fn sqlstate(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .map(|code| code.into_owned())
}

/// The SQLSTATE of `script` run by `pool` in one transaction; `None` when it commits.
async fn refused(pool: &PgPool, script: &str) -> TestResult<Option<String>> {
    let mut tx = pool.begin().await?;
    if let Err(error) = sqlx::raw_sql(sqlx::AssertSqlSafe(script.to_owned()))
        .execute(&mut *tx)
        .await
    {
        return Ok(sqlstate(&error));
    }
    Ok(tx.commit().await.err().as_ref().and_then(sqlstate))
}

#[test]
fn gains_and_spends_chain_one_entry_per_change() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "chain").await?;
        harness.standalone_spends().await?;
        assert_eq!(harness.balance().await?, ForgeDustBalance::default());
        assert_eq!(written(commit(&harness, Op::Gain(1, 30)).await?)?, (30, 0));
        assert_eq!(written(commit(&harness, Op::Spend(2, 12)).await?)?, (18, 0));
        assert_eq!(written(commit(&harness, Op::Gain(3, 7)).await?)?, (25, 0));
        assert_eq!(harness.balance().await?, balance(25)?);
        assert_eq!(harness.entries().await?, 3);
        let audit = harness.audit().await?;
        assert_eq!(
            (audit.entries, audit.credited, audit.lost, audit.spent),
            (3, 37, 0, 12)
        );
        assert_eq!(audit.balance, balance(25)?);

        // The caller's transaction carries the entry: a rollback writes nothing.
        let mut tx = harness.runtime.begin().await?;
        lock_root(&mut tx).await?;
        written(write(&mut tx, WORLD, Op::Gain(4, 5)).await?)?;
        tx.rollback().await?;
        assert_eq!(harness.entries().await?, 3);
        harness.cleanup().await
    })
}

#[test]
fn gain_above_limit_credits_to_limit_and_records_the_lost_part() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "lost").await?;
        written(commit(&harness, Op::Gain(1, 90)).await?)?;
        assert_eq!(
            written(commit(&harness, Op::Gain(2, 25)).await?)?,
            (100, 15)
        );
        // At the limit a gain credits nothing; it still has its entry with the whole amount lost.
        assert_eq!(
            written(commit(&harness, Op::Gain(3, 40)).await?)?,
            (100, 40)
        );
        let audit = harness.audit().await?;
        assert_eq!(
            (audit.entries, audit.credited, audit.lost, audit.spent),
            (3, 100, 55, 0)
        );
        let stored: (i64, i64, i64) = sqlx::query_as(
            "SELECT amount, lost_amount, balance_after FROM game_character_forge_dust_entries \
              WHERE entry_id = encode($1,'hex')::uuid",
        )
        .bind(id(102).as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(stored, (25, 15, 100));
        harness.cleanup().await
    })
}

#[test]
fn spend_above_balance_is_refused_before_any_write() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "refuse").await?;
        harness.standalone_spends().await?;
        // No row: balance 0, refused, and no zero row is written.
        let mut tx = harness.runtime.begin().await?;
        lock_root(&mut tx).await?;
        assert!(matches!(
            write(&mut tx, WORLD, Op::Spend(1, 1)).await?,
            Err(ForgeDustWriteError::InsufficientDust)
        ));
        let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_forge_dust")
            .fetch_one(&mut *tx)
            .await?;
        assert_eq!(rows, 0);
        tx.rollback().await?;

        written(commit(&harness, Op::Gain(2, 30)).await?)?;
        let mut tx = harness.runtime.begin().await?;
        lock_root(&mut tx).await?;
        assert!(matches!(
            write(&mut tx, WORLD, Op::Spend(3, 31)).await?,
            Err(ForgeDustWriteError::InsufficientDust)
        ));
        let (entries, stored): (i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_character_forge_dust_entries), \
                    (SELECT balance FROM game_character_forge_dust)",
        )
        .fetch_one(&mut *tx)
        .await?;
        assert_eq!((entries, stored), (1, 30));
        tx.rollback().await?;
        // The whole balance can be spent.
        assert_eq!(written(commit(&harness, Op::Spend(4, 30)).await?)?, (0, 0));
        assert!(matches!(
            commit(&harness, Op::Gain(5, 0)).await?,
            Err(ForgeDustWriteError::InvalidInput)
        ));
        harness.cleanup().await
    })
}

#[test]
fn occurrence_replays_its_entry_and_conflicts_on_another_binding() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "replay").await?;
        written(commit(&harness, Op::Gain(1, 30)).await?)?;
        match commit(&harness, Op::Gain(1, 30)).await? {
            Ok(ForgeDustOutcome::AlreadyWritten(entry)) => {
                assert_eq!(entry.kind, ForgeDustEntryKind::Gain);
                assert_eq!((entry.amount, entry.after.balance()), (30, 30));
                assert_eq!(entry.entry_id, id(101));
            }
            other => return Err(format!("expected a replay, got {other:?}").into()),
        }
        assert!(matches!(
            commit(&harness, Op::Gain(1, 31)).await?,
            Err(ForgeDustWriteError::ConflictingOccurrence)
        ));
        assert_eq!(harness.entries().await?, 1);
        assert_eq!(harness.balance().await?, balance(30)?);
        // A fence of another World is refused.
        let mut tx = harness.runtime.begin().await?;
        assert!(matches!(
            write(&mut tx, OTHER_WORLD, Op::Gain(2, 1)).await?,
            Err(ForgeDustWriteError::CharacterMismatch)
        ));
        tx.rollback().await?;
        harness.cleanup().await
    })
}

#[test]
fn ledger_is_immutable_and_the_runtime_never_deletes() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "immutable").await?;
        written(commit(&harness, Op::Gain(1, 30)).await?)?;
        let entry = format!("'{}'::uuid", hex(&id(101)));
        for script in [
            format!(
                "UPDATE game_character_forge_dust_entries SET amount = 31 WHERE entry_id = {entry}"
            ),
            format!("DELETE FROM game_character_forge_dust_entries WHERE entry_id = {entry}"),
            "TRUNCATE game_character_forge_dust_entries CASCADE".to_owned(),
            "TRUNCATE game_character_forge_dust".to_owned(),
            "DELETE FROM game_character_forge_dust".to_owned(),
        ] {
            assert_eq!(
                refused(&harness.pool, &script).await?.as_deref(),
                Some("23514"),
                "owner: {script}"
            );
        }
        for script in [
            format!(
                "UPDATE game_character_forge_dust_entries SET amount = 31 WHERE entry_id = {entry}"
            ),
            format!("DELETE FROM game_character_forge_dust_entries WHERE entry_id = {entry}"),
            "DELETE FROM game_character_forge_dust".to_owned(),
            "TRUNCATE game_character_forge_dust".to_owned(),
        ] {
            assert_eq!(
                refused(&harness.runtime, &script).await?.as_deref(),
                Some("42501"),
                "runtime: {script}"
            );
        }
        // Grants as BANK-0 §3.
        let grants: Vec<(String, String)> = sqlx::query_as(
            "SELECT table_name::text, privilege_type::text \
               FROM information_schema.role_table_grants \
              WHERE grantee = 'oteryn_game_runtime' \
                AND table_name IN ('game_character_forge_dust', \
                                   'game_character_forge_dust_entries') \
              ORDER BY 1, 2",
        )
        .fetch_all(&harness.pool)
        .await?;
        let expected = [
            ("game_character_forge_dust", "INSERT"),
            ("game_character_forge_dust", "SELECT"),
            ("game_character_forge_dust", "UPDATE"),
            ("game_character_forge_dust_entries", "INSERT"),
            ("game_character_forge_dust_entries", "SELECT"),
        ]
        .map(|(table, privilege)| (table.to_owned(), privilege.to_owned()));
        assert_eq!(grants, expected);
        assert_eq!(harness.audit().await?.balance, balance(30)?);
        harness.cleanup().await
    })
}

#[test]
fn forged_balance_or_chain_is_refused() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "forged").await?;
        written(commit(&harness, Op::Gain(1, 30)).await?)?;
        let root = uuid(CHARACTER);
        let entry = |seed: u8, previous: &str, before: u16, after: u16| {
            format!(
                "INSERT INTO game_character_forge_dust_entries (entry_id, character_id, \
                   previous_entry_id, kind, cause, cause_occurrence_id, transaction_id, amount, \
                   lost_amount, balance_before, balance_after, dust_limit_before, \
                   dust_limit_after) \
                 VALUES ({}, {root}, {previous}, 'GAIN', 'creature_kill', {}, {}, {}, 0, \
                   {before}, {after}, 100, 100);",
                uuid(seed + 100),
                uuid(seed),
                uuid(seed + 130),
                after - before,
            )
        };
        let previous = uuid(101);
        let update = |after: u16, last: u8| {
            format!(
                "UPDATE game_character_forge_dust SET balance = {after}, last_entry_id = {} \
                  WHERE character_id = {root};",
                uuid(last + 100)
            )
        };
        for (case, script) in [
            (
                "balance without an entry",
                "UPDATE game_character_forge_dust SET balance = 99;".to_owned(),
            ),
            ("entry without the balance row", entry(2, &previous, 30, 40)),
            (
                "entry not following its predecessor",
                format!("{}{}", entry(2, &previous, 20, 40), update(40, 2)),
            ),
            (
                "second first entry",
                format!("{}{}", entry(2, "NULL", 0, 40), update(40, 2)),
            ),
            (
                "row not equal to its entry",
                format!("{}{}", entry(2, &previous, 30, 40), update(41, 2)),
            ),
            (
                "arithmetic",
                format!(
                    "{}{}",
                    entry(2, &previous, 30, 40).replace(", 10, 0,", ", 11, 0,"),
                    update(40, 2)
                ),
            ),
            (
                "unadmitted kind",
                format!(
                    "{}{}",
                    entry(2, &previous, 30, 40).replace("'GAIN'", "'CONVERT'"),
                    update(40, 2)
                ),
            ),
            (
                "cause of another kind",
                format!(
                    "{}{}",
                    entry(2, &previous, 30, 40).replace("'creature_kill'", "'proficiency'"),
                    update(40, 2)
                ),
            ),
        ] {
            let expected = if case == "second first entry" {
                "23505"
            } else {
                "23514"
            };
            assert_eq!(
                refused(&harness.runtime, &script).await?.as_deref(),
                Some(expected),
                "{case}"
            );
        }
        // The row cannot point back to an older entry once a newer one exists.
        written(commit(&harness, Op::Gain(3, 5)).await?)?;
        assert_eq!(
            refused(&harness.runtime, &update(30, 1)).await?.as_deref(),
            Some("23514")
        );
        // A forged chain committed with the guards off is found by the audit.
        harness
            .seed(&format!(
                "{}{}",
                entry(4, &uuid(103), 20, 40),
                update(40, 4)
            ))
            .await?;
        let mut connection = harness.pool.acquire().await?;
        assert!(matches!(
            audit_forge_dust_ledger(&mut connection, character()?, 1_000).await,
            Err(ForgeDustAuditError::Inconsistent)
        ));
        assert!(matches!(
            audit_forge_dust_ledger(&mut connection, character()?, 2).await,
            Err(ForgeDustAuditError::TooManyEntries)
        ));
        drop(connection);
        harness.cleanup().await
    })
}

#[test]
fn dust_limit_check_accepts_100_and_225_and_rejects_99_and_226() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "limit").await?;
        // `IMBFORGE0-RL-08`, the CHECK alone: the guards and foreign keys are off.
        for (limit, accepted) in [(99, false), (100, true), (225, true), (226, false)] {
            let mut tx = harness.pool.begin().await?;
            tx.execute("SET LOCAL session_replication_role = replica")
                .await?;
            let row = tx
                .execute(sqlx::AssertSqlSafe(format!(
                    "INSERT INTO game_character_forge_dust VALUES \
                       ({}, 0, {limit}, {})",
                    uuid(CHARACTER),
                    uuid(200),
                )))
                .await;
            assert_eq!(row.is_ok(), accepted, "row limit {limit}");
            if let Err(error) = row {
                assert_eq!(sqlstate(&error).as_deref(), Some("23514"));
                assert!(
                    error
                        .to_string()
                        .contains("game_character_forge_dust_limit_range"),
                    "{error}"
                );
            }
            tx.rollback().await?;

            let mut tx = harness.pool.begin().await?;
            tx.execute("SET LOCAL session_replication_role = replica")
                .await?;
            let entry = tx
                .execute(sqlx::AssertSqlSafe(format!(
                    "INSERT INTO game_character_forge_dust_entries (entry_id, character_id, \
                       kind, cause, cause_occurrence_id, transaction_id, amount, lost_amount, \
                       balance_before, balance_after, dust_limit_before, dust_limit_after) \
                     VALUES ({}, {}, 'GAIN', 'creature_kill', {}, {}, 1, 0, 0, 1, \
                       {limit}, {limit})",
                    uuid(201),
                    uuid(CHARACTER),
                    uuid(202),
                    uuid(203),
                )))
                .await;
            assert_eq!(entry.is_ok(), accepted, "entry limit {limit}");
            tx.rollback().await?;
        }
        harness.cleanup().await
    })
}

#[test]
fn writer_locks_the_root_before_the_dust_row_and_writers_serialize() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "locks").await?;
        harness.standalone_spends().await?;
        written(commit(&harness, Op::Gain(1, 10)).await?)?;

        // A transaction holding only the root lock (rule 4) blocks the writer before it reaches
        // the dust row: the dust row stays free meanwhile.
        let mut holder = harness.runtime.begin().await?;
        lock_root(&mut holder).await?;
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let runtime = harness.runtime.clone();
        let waiting = done.clone();
        let hold = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let waited = !waiting.load(std::sync::atomic::Ordering::SeqCst);
            let mut probe = runtime.begin().await.map_err(|error| error.to_string())?;
            let free: Option<i64> = sqlx::query_scalar(
                "SELECT balance FROM game_character_forge_dust FOR UPDATE NOWAIT",
            )
            .fetch_optional(&mut *probe)
            .await
            .map_err(|error| error.to_string())?;
            probe.rollback().await.map_err(|error| error.to_string())?;
            // The concurrent gain commits first; the waiting spend then sees its balance.
            let fence = fence_in(WORLD).map_err(|error| error.to_string())?;
            let cause = ForgeDustGainCause::CreatureKill { occurrence: id(3) };
            let gain = gain_forge_dust_in_transaction(&mut holder, &fence, cause, 15, ids(3))
                .await
                .map_err(|error| error.to_string())?;
            holder.commit().await.map_err(|error| error.to_string())?;
            Ok::<_, String>((waited, free, gain))
        });
        let mut tx = harness.runtime.begin().await?;
        let spend = write(&mut tx, WORLD, Op::Spend(2, 20)).await?;
        done.store(true, std::sync::atomic::Ordering::SeqCst);
        tx.commit().await?;
        let (waited, free, gain) = hold.await.map_err(debug)??;
        assert!(waited, "the writer waits on the root lock");
        assert_eq!(free, Some(10), "the dust row is not locked before the root");
        assert_eq!(written(Ok(gain))?, (25, 0));
        assert_eq!(written(spend)?, (5, 0));

        let audit = harness.audit().await?;
        assert_eq!((audit.entries, audit.credited, audit.spent), (3, 25, 20));
        assert_eq!(audit.balance, balance(5)?);
        harness.cleanup().await
    })
}
