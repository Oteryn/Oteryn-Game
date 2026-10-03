// Shared TIMED-RT-1a cases (migration 0054). Any wrapper that provides the same path-loaded crate
// root as `item_timed_state_postgres.rs` can include this file.
//
// The hosting runtime's fenced one-item transaction does not exist yet (TIMED-RT-1b), so each case
// runs `checkpoint_in_transaction` in a runtime-role transaction of its own. The fixture
// (character, main backpack, timed items) is seeded by the migration owner with the guards off,
// as other item cases do.

use crate::domain::CharacterId;
use crate::domain::timed_item::TimedValues;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_timed_state::{
    StoredTimedState, TimedCheckpointRequest, TimedDefinitionFacts, TimedItemCause,
    TimedWriteCause, TimedWriteError, TimedWriteOutcome, checkpoint_in_transaction,
    find_timed_write, load_timed_state,
};
use sqlx::{Connection, Executor, PgPool};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CHARACTER: u8 = 61;
const OTHER_CHARACTER: u8 = 62;
const ACCOUNT: u8 = 64;
const WORLD: u8 = 63;
const BACKPACK: u8 = 70;
const RING: u8 = 80;
const AMULET: u8 = 90;
const TORCH: u8 = 100;
const GROUND_RING: u8 = 110;
const RING_KEY: &str = "oteryn:item.tibia.i3052";
const AMULET_KEY: &str = "oteryn:item.tibia.i3081";
const TORCH_KEY: &str = "oteryn:item.tibia.i2051";
const BACKPACK_KEY: &str = "oteryn:item.tibia.i2854";
const RING_MS: u64 = 1_200_000;
const AMULET_CHARGES: u32 = 200;

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
    /// A login in the runtime group (0006): every timed transaction runs as it.
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
        let name = format!("timed_{tag}_{suffix}");
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

        let role = format!("timed_runtime_{tag}_{suffix}");
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
        let mut script = String::new();
        for (character, name) in [(CHARACTER, "Timed Hero"), (OTHER_CHARACTER, "Other Hero")] {
            script.push_str(&format!(
                "INSERT INTO game_character_roots VALUES ({}, {}, {}, 1, 1, 'profile-1', \
                   'ruleset-1', 'content-1', 'starter-1', '{name}');",
                uuid(character),
                uuid(ACCOUNT),
                uuid(WORLD),
            ));
        }
        script.push_str(&item(BACKPACK, BACKPACK_KEY));
        script.push_str(&format!(
            "INSERT INTO game_item_container_slots(character_id, item_instance_id, world_id, \
               placed_transaction_id) VALUES ({}, {}, {}, {});",
            uuid(CHARACTER),
            uuid(BACKPACK),
            uuid(WORLD),
            uuid(BACKPACK + 2),
        ));
        for (seed, key, ordinal) in [
            (RING, RING_KEY, 1),
            (AMULET, AMULET_KEY, 2),
            (TORCH, TORCH_KEY, 3),
        ] {
            script.push_str(&item(seed, key));
            script.push_str(&format!(
                "INSERT INTO game_item_container_entries(item_instance_id, world_id, \
                   character_id, parent_item_instance_id, placement_ordinal, \
                   placed_transaction_id) VALUES ({}, {}, {}, {}, {ordinal}, {});",
                uuid(seed),
                uuid(WORLD),
                uuid(CHARACTER),
                uuid(BACKPACK),
                uuid(seed + 2),
            ));
        }
        // A ring lying on Ground: held by nobody.
        script.push_str(&item(GROUND_RING, RING_KEY));
        script.push_str(&format!(
            "INSERT INTO game_item_ground_locations VALUES ({}, {}, {}, 1, '\\x01'::bytea, \
               '\\x01'::bytea, 'map-1', 'content-1', '\\x01'::bytea);",
            uuid(GROUND_RING),
            uuid(WORLD),
            uuid(WORLD + 100),
        ));
        harness.seed(&script).await?;
        Ok(harness)
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

    /// Raw statements as the runtime role, committed; the SQLSTATE of a refusal.
    async fn runtime_sql(&self, script: String) -> TestResult<Option<String>> {
        let mut tx = self.runtime.begin().await?;
        if let Err(error) = sqlx::raw_sql(sqlx::AssertSqlSafe(script))
            .execute(&mut *tx)
            .await
        {
            tx.rollback().await?;
            return Ok(Some(sqlstate(&error)));
        }
        match tx.commit().await {
            Ok(()) => Ok(None),
            Err(error) => Ok(Some(sqlstate(&error))),
        }
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

/// A live item of `key`. Item `seed` uses the ids `seed` (item), `seed + 1` (minted) and
/// `seed + 2` (placed).
fn item(seed: u8, key: &str) -> String {
    format!(
        "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
           definition_production_key, definition_revision_ref, quantity, lifecycle, \
           minted_transaction_id) \
         VALUES ({}, {}, 'Item', '{key}', 'rev-1', 1, 1, {});",
        uuid(seed),
        uuid(WORLD),
        uuid(seed + 1),
    )
}

fn holder(seed: u8) -> TestResult<CharacterId> {
    CharacterId::from_bytes(id(seed)).map_err(debug)
}

fn values(charges: Option<u32>, remaining_ms: Option<u64>) -> TestResult<TimedValues> {
    TimedValues::new(charges, remaining_ms).map_err(debug)
}

fn facts(key: &str, full: TimedValues, lit_continuous: bool) -> TimedDefinitionFacts {
    TimedDefinitionFacts {
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "rev-1".into(),
        },
        full,
        lit_continuous,
    }
}

fn ring(expected_revision: u64, tx: u8, remaining_ms: u64) -> TestResult<TimedCheckpointRequest> {
    Ok(TimedCheckpointRequest {
        item_instance_id: id(RING),
        expected_revision,
        transaction_id: id(tx),
        definition: facts(RING_KEY, values(None, Some(RING_MS))?, false),
        values: values(None, Some(remaining_ms))?,
    })
}

#[derive(Debug)]
enum Checkpointed {
    Committed(TimedWriteOutcome),
    Refused(TimedWriteError),
    /// The SQLSTATE of a refused commit.
    CommitFailed(String),
}

async fn checkpoint(
    harness: &Harness,
    holder_seed: u8,
    request: &TimedCheckpointRequest,
) -> TestResult<Checkpointed> {
    let mut tx = harness.runtime.begin().await?;
    match checkpoint_in_transaction(&mut tx, holder(holder_seed)?, request).await {
        Ok(outcome) => match tx.commit().await {
            Ok(()) => Ok(Checkpointed::Committed(outcome)),
            Err(error) => Ok(Checkpointed::CommitFailed(sqlstate(&error))),
        },
        Err(error) => {
            tx.rollback().await?;
            Ok(Checkpointed::Refused(error))
        }
    }
}

async fn state(harness: &Harness, seed: u8) -> TestResult<Option<StoredTimedState>> {
    let mut connection = harness.pool.acquire().await?;
    load_timed_state(&mut connection, &id(seed))
        .await
        .map_err(debug)
}

async fn records(pool: &PgPool) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_item_timed_state_writes")
            .fetch_one(pool)
            .await?,
    )
}

fn sqlstate(error: &sqlx::Error) -> String {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .map_or_else(|| format!("{error}"), |code| code.into_owned())
}

/// A raw record of the ring definition for `item` at `expected`, cause `cause`, with the given facts and values; the
/// row write follows in `row`.
fn raw_record(
    item_seed: u8,
    expected: u64,
    cause: i16,
    tx: u8,
    full: (&str, &str, bool),
    before: (&str, &str),
    after: (&str, &str, &str),
) -> String {
    format!(
        "INSERT INTO game_item_timed_state_writes (item_instance_id, expected_revision, cause, \
           transaction_id, holder_character_id, definition_before_family, \
           definition_before_production_key, definition_before_revision_ref, \
           definition_after_family, definition_after_production_key, \
           definition_after_revision_ref, full_charges, full_remaining_ms, lit_continuous, \
           charges_before, remaining_ms_before, deadline_before, charges_after, \
           remaining_ms_after, deadline_after) \
         VALUES ({item}, {expected}, {cause}, {tx}, {holder}, 'Item', '{RING_KEY}', 'rev-1', \
           'Item', '{RING_KEY}', 'rev-1', {fc}, {fr}, {lit}, {cb}, {rb}, NULL, {ca}, {ra}, {da});",
        item = uuid(item_seed),
        tx = uuid(tx),
        holder = uuid(CHARACTER),
        fc = full.0,
        fr = full.1,
        lit = full.2,
        cb = before.0,
        rb = before.1,
        ca = after.0,
        ra = after.1,
        da = after.2,
    )
}

#[test]
fn a_first_checkpoint_inserts_at_zero_and_a_replay_returns_its_result() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "first").await?;
        assert_eq!(state(&harness, RING).await?, None);

        let first = ring(0, 1, RING_MS - 59_000)?;
        match checkpoint(&harness, CHARACTER, &first).await? {
            Checkpointed::Committed(TimedWriteOutcome::Written { revision: 1 }) => {}
            other => return Err(format!("first checkpoint: {other:?}").into()),
        }
        let stored = state(&harness, RING).await?.ok_or("no row")?;
        assert_eq!(stored.revision, 1);
        assert_eq!(stored.values, Some(values(None, Some(RING_MS - 59_000))?));
        assert_eq!(stored.deadline_at, None);

        let mut connection = harness.pool.acquire().await?;
        let record = find_timed_write(&mut connection, &id(RING), 0)
            .await
            .map_err(debug)?
            .ok_or("no record")?;
        drop(connection);
        assert_eq!(
            record.cause,
            TimedWriteCause::Timed(TimedItemCause::Checkpoint)
        );
        assert_eq!(record.transaction_id, id(1));
        assert_eq!(record.after, Some(values(None, Some(RING_MS - 59_000))?));
        assert_eq!(record.deadline_after, None);
        assert!(record.committed_at > 0);

        // The exact replay returns the recorded result and writes nothing.
        match checkpoint(&harness, CHARACTER, &first).await? {
            Checkpointed::Committed(TimedWriteOutcome::AlreadyCommitted { revision: 1 }) => {}
            other => return Err(format!("replay: {other:?}").into()),
        }
        // A second insert at expected revision 0 writes nothing.
        match checkpoint(&harness, CHARACTER, &ring(0, 2, RING_MS - 60_000)?).await? {
            Checkpointed::Refused(TimedWriteError::ConflictingWrite) => {}
            other => return Err(format!("second insert at 0: {other:?}").into()),
        }
        assert_eq!(records(&harness.pool).await?, 1);

        // The next checkpoint moves the row up by one from the stored value.
        match checkpoint(&harness, CHARACTER, &ring(1, 3, RING_MS - 119_000)?).await? {
            Checkpointed::Committed(TimedWriteOutcome::Written { revision: 2 }) => {}
            other => return Err(format!("second checkpoint: {other:?}").into()),
        }
        assert_eq!(state(&harness, RING).await?.ok_or("no row")?.revision, 2);
        harness.cleanup().await
    })
}

#[test]
fn refused_checkpoints_write_nothing() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "refuse").await?;
        match checkpoint(&harness, CHARACTER, &ring(0, 1, RING_MS - 1_000)?).await? {
            Checkpointed::Committed(_) => {}
            other => return Err(format!("setup checkpoint: {other:?}").into()),
        }
        // Unchanged values write nothing; a value above the row never adds time.
        for (remaining, label) in [(RING_MS - 1_000, "unchanged"), (RING_MS - 500, "higher")] {
            match checkpoint(&harness, CHARACTER, &ring(1, 2, remaining)?).await? {
                Checkpointed::Refused(TimedWriteError::NotAStoreableChange) => {}
                other => return Err(format!("{label}: {other:?}").into()),
            }
        }
        // A stale or future expected revision finds another revision.
        match checkpoint(&harness, CHARACTER, &ring(5, 3, RING_MS - 2_000)?).await? {
            Checkpointed::Refused(TimedWriteError::RevisionMismatch {
                current: Some(StoredTimedState { revision: 1, .. }),
            }) => {}
            other => return Err(format!("revision mismatch: {other:?}").into()),
        }
        // Another character does not hold the ring; nobody holds a Ground item.
        match checkpoint(&harness, OTHER_CHARACTER, &ring(1, 4, RING_MS - 2_000)?).await? {
            Checkpointed::Refused(TimedWriteError::NotHeld) => {}
            other => return Err(format!("other holder: {other:?}").into()),
        }
        let mut ground = ring(0, 5, RING_MS - 2_000)?;
        ground.item_instance_id = id(GROUND_RING);
        match checkpoint(&harness, CHARACTER, &ground).await? {
            Checkpointed::Refused(TimedWriteError::NotHeld) => {}
            other => return Err(format!("ground ring: {other:?}").into()),
        }
        // Values outside the definition are invalid input.
        let mut wrong_shape = ring(1, 6, RING_MS - 2_000)?;
        wrong_shape.values = values(Some(3), Some(RING_MS - 2_000))?;
        match checkpoint(&harness, CHARACTER, &wrong_shape).await? {
            Checkpointed::Refused(TimedWriteError::InvalidInput) => {}
            other => return Err(format!("wrong shape: {other:?}").into()),
        }
        assert_eq!(records(&harness.pool).await?, 1);
        assert_eq!(state(&harness, RING).await?.ok_or("no row")?.revision, 1);
        harness.cleanup().await
    })
}

#[test]
fn a_charges_checkpoint_stores_the_live_charges() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "charges").await?;
        let request = TimedCheckpointRequest {
            item_instance_id: id(AMULET),
            expected_revision: 0,
            transaction_id: id(1),
            definition: facts(AMULET_KEY, values(Some(AMULET_CHARGES), None)?, false),
            values: values(Some(AMULET_CHARGES - 7), None)?,
        };
        match checkpoint(&harness, CHARACTER, &request).await? {
            Checkpointed::Committed(TimedWriteOutcome::Written { revision: 1 }) => {}
            other => return Err(format!("charges checkpoint: {other:?}").into()),
        }
        assert_eq!(
            state(&harness, AMULET).await?.ok_or("no row")?.values,
            Some(values(Some(AMULET_CHARGES - 7), None)?)
        );
        harness.cleanup().await
    })
}

#[test]
fn the_guard_refuses_a_lit_item_in_a_container_and_other_full_values() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "lit").await?;
        // A lit torch in a backpack entry: the guard refuses it at commit (D360).
        let torch = TimedCheckpointRequest {
            item_instance_id: id(TORCH),
            expected_revision: 0,
            transaction_id: id(1),
            definition: facts(TORCH_KEY, values(None, Some(600_000))?, true),
            values: values(None, Some(500_000))?,
        };
        match checkpoint(&harness, CHARACTER, &torch).await? {
            Checkpointed::CommitFailed(state) if state == "23514" => {}
            other => return Err(format!("lit torch in a bag: {other:?}").into()),
        }
        // The first write pins a definition's full values; a later one claiming others is
        // refused.
        match checkpoint(&harness, CHARACTER, &ring(0, 2, RING_MS - 1_000)?).await? {
            Checkpointed::Committed(_) => {}
            other => return Err(format!("pin: {other:?}").into()),
        }
        let mut inflated = ring(1, 3, RING_MS - 2_000)?;
        inflated.definition.full = values(None, Some(RING_MS * 2))?;
        match checkpoint(&harness, CHARACTER, &inflated).await? {
            Checkpointed::CommitFailed(state) if state == "23514" => {}
            other => return Err(format!("inflated full values: {other:?}").into()),
        }
        assert_eq!(records(&harness.pool).await?, 1);
        harness.cleanup().await
    })
}

#[test]
fn the_guard_refuses_writes_without_their_record_or_row() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "guard").await?;
        let ring_full = (String::from("NULL"), RING_MS.to_string(), false);
        let full = (ring_full.0.as_str(), ring_full.1.as_str(), ring_full.2);
        let before = ("NULL", ring_full.1.as_str());
        // A row without its record.
        let refused = harness
            .runtime_sql(format!(
                "INSERT INTO game_item_timed_states VALUES ({}, NULL, 1000, NULL, 1);",
                uuid(RING)
            ))
            .await?;
        assert_eq!(refused.as_deref(), Some("23514"), "row without record");
        // A record without its row.
        let refused = harness
            .runtime_sql(raw_record(
                RING,
                0,
                1,
                1,
                full,
                before,
                ("NULL", "1000", "NULL"),
            ))
            .await?;
        assert_eq!(refused.as_deref(), Some("23514"), "record without row");
        // A cause 0054 does not admit (Expire), even with its row.
        let refused = harness
            .runtime_sql(format!(
                "{} INSERT INTO game_item_timed_states VALUES ({}, NULL, 1000, NULL, 1);",
                raw_record(RING, 0, 2, 2, full, before, ("NULL", "1000", "NULL")),
                uuid(RING)
            ))
            .await?;
        assert_eq!(refused.as_deref(), Some("23514"), "unadmitted cause");
        // A deadline on an item that is not a lit item on a tile.
        let refused = harness
            .runtime_sql(format!(
                "{} INSERT INTO game_item_timed_states VALUES ({}, NULL, 1000, 99999999999, 1);",
                raw_record(RING, 0, 1, 3, full, before, ("NULL", "1000", "99999999999")),
                uuid(RING)
            ))
            .await?;
        assert_eq!(refused.as_deref(), Some("23514"), "deadline on a held item");
        // A row created at revision 2.
        let refused = harness
            .runtime_sql(format!(
                "{} INSERT INTO game_item_timed_states VALUES ({}, NULL, 1000, NULL, 2);",
                raw_record(RING, 0, 1, 4, full, before, ("NULL", "1000", "NULL")),
                uuid(RING)
            ))
            .await?;
        assert_eq!(refused.as_deref(), Some("23514"), "row created at 2");
        assert_eq!(records(&harness.pool).await?, 0);

        // The admitted shape commits through the raw SQL too.
        let committed = harness
            .runtime_sql(format!(
                "{} INSERT INTO game_item_timed_states VALUES ({}, NULL, 1000, NULL, 1);",
                raw_record(RING, 0, 1, 5, full, before, ("NULL", "1000", "NULL")),
                uuid(RING)
            ))
            .await?;
        assert_eq!(committed, None, "admitted checkpoint");
        // A revision jump, a delete, a truncate and a record change are refused.
        for (statement, label) in [
            (
                format!(
                    "{} UPDATE game_item_timed_states SET remaining_ms = 900, \
                       state_revision = 3 WHERE item_instance_id = {};",
                    raw_record(
                        RING,
                        1,
                        1,
                        6,
                        full,
                        ("NULL", "1000"),
                        ("NULL", "900", "NULL")
                    ),
                    uuid(RING)
                ),
                "revision +2",
            ),
            (
                format!(
                    "DELETE FROM game_item_timed_states WHERE item_instance_id = {};",
                    uuid(RING)
                ),
                "delete",
            ),
            (
                format!(
                    "UPDATE game_item_timed_state_writes SET remaining_ms_after = 1 \
                      WHERE item_instance_id = {};",
                    uuid(RING)
                ),
                "record change",
            ),
            (
                String::from("TRUNCATE game_item_timed_state_writes;"),
                "truncate",
            ),
        ] {
            let refused = harness.runtime_sql(statement).await?;
            assert!(refused.is_some(), "{label} must be refused");
        }
        assert_eq!(state(&harness, RING).await?.ok_or("no row")?.revision, 1);
        assert_eq!(records(&harness.pool).await?, 1);
        harness.cleanup().await
    })
}
