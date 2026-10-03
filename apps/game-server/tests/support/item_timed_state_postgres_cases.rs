// Shared TIMED-RT-1a cases (migration 0054). Any wrapper that provides the same path-loaded crate
// root as `item_timed_state_postgres.rs`, with the Bestiary harness (one bootstrapped Character
// 41 on a live Channel session in World 42 / Channel 43, held by node 1 at scope ownership
// generation 1), can include this file.
//
// The timed items (a ring, a charged amulet and a lit torch in the main backpack, and a ring on
// Ground) are seeded by the migration owner with the guards off, as other item cases do. Every
// checkpoint goes through `DurabilityRoot::commit_timed_checkpoint` under the Character's current
// gameplay fence; the raw guard cases run as a login in the runtime group.

use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, WORLD, configured_admin, debug, fence, id, runtime,
};
use crate::domain::timed_item::TimedValues;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_timed_state::{
    StoredTimedState, TimedCheckpointRequest, TimedDefinitionFacts, TimedItemCause,
    TimedWriteCause, TimedWriteError, TimedWriteOutcome, find_timed_write, load_timed_state,
};
use sqlx::PgPool;

const BACKPACK: u8 = 70;
const RING: u8 = 80;
const AMULET: u8 = 90;
const TORCH: u8 = 100;
const GROUND_RING: u8 = 110;
const OTHER_RING: u8 = 120;
const RING_KEY: &str = "oteryn:item.tibia.i3052";
const AMULET_KEY: &str = "oteryn:item.tibia.i3081";
const TORCH_KEY: &str = "oteryn:item.tibia.i2051";
const BACKPACK_KEY: &str = "oteryn:item.tibia.i2854";
const RING_MS: u64 = 1_200_000;
const AMULET_CHARGES: u32 = 200;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn uuid(seed: u8) -> String {
    format!("'{}'::uuid", hex(&id(seed)))
}

/// The Bestiary harness plus the timed fixture and a login in the runtime group.
struct Timed {
    harness: Harness,
    runtime: PgPool,
}

impl Timed {
    async fn create(admin: String, tag: &str) -> TestResult<Self> {
        let harness = Harness::create(admin, tag, false).await?;
        let mut script = String::new();
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
            script.push_str(&entry(seed, BACKPACK, ordinal));
        }
        // A ring in a bag that is in nobody's custody.
        script.push_str(&item(OTHER_RING, RING_KEY));
        // A ring lying on Ground: held by nobody.
        script.push_str(&item(GROUND_RING, RING_KEY));
        script.push_str(&format!(
            "INSERT INTO game_item_ground_locations VALUES ({}, {}, {}, 1, '\\x01'::bytea, \
               '\\x01'::bytea, 'map-1', 'content-1', '\\x01'::bytea);",
            uuid(GROUND_RING),
            uuid(WORLD),
            uuid(43),
        ));
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(script))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let role = format!("timed_runtime_{tag}_{suffix}");
        let password = format!("{role}-secret");
        let database: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&harness.pool)
            .await?;
        for statement in [
            format!("CREATE ROLE {role} LOGIN PASSWORD '{password}' IN ROLE oteryn_game_runtime"),
            format!("GRANT CONNECT ON DATABASE {database} TO {role}"),
        ] {
            sqlx::query(sqlx::AssertSqlSafe(statement))
                .execute(&harness.pool)
                .await?;
        }
        let (_, address) = harness
            .database
            .url
            .split_once('@')
            .ok_or("no authority separator")?;
        let runtime = PgPool::connect(&format!("postgresql://{role}:{password}@{address}")).await?;
        Ok(Self { harness, runtime })
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

    /// One checkpoint under the Character's fence at `revision`.
    async fn checkpoint(
        &self,
        revision: u64,
        request: &TimedCheckpointRequest,
    ) -> Result<TimedWriteOutcome, TimedWriteError> {
        let seal = self
            .harness
            .recovery
            .seal_current()
            .map_err(|_| TimedWriteError::AuthorityRejected)?;
        let authority = self
            .harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|_| TimedWriteError::AuthorityRejected)?;
        let fence = fence(revision).map_err(|_| TimedWriteError::InvalidInput)?;
        self.harness
            .root
            .commit_timed_checkpoint(&authority, &self.harness.node, fence, request.clone())
            .await
    }

    async fn state(&self, seed: u8) -> TestResult<Option<StoredTimedState>> {
        let mut connection = self.harness.pool.acquire().await?;
        load_timed_state(&mut connection, &id(seed))
            .await
            .map_err(|error| debug(error).into())
    }

    async fn records(&self) -> TestResult<i64> {
        self.harness.count("game_item_timed_state_writes").await
    }

    async fn cleanup(self) -> TestResult {
        self.runtime.close().await;
        self.harness.cleanup().await
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

fn entry(seed: u8, parent: u8, ordinal: u64) -> String {
    format!(
        "INSERT INTO game_item_container_entries(item_instance_id, world_id, character_id, \
           parent_item_instance_id, placement_ordinal, placed_transaction_id) \
         VALUES ({}, {}, {}, {}, {ordinal}, {});",
        uuid(seed),
        uuid(WORLD),
        uuid(CHARACTER),
        uuid(parent),
        uuid(seed + 2),
    )
}

fn values(charges: Option<u32>, remaining_ms: Option<u64>) -> TestResult<TimedValues> {
    TimedValues::new(charges, remaining_ms).map_err(|error| debug(error).into())
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

fn sqlstate(error: &sqlx::Error) -> String {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .map_or_else(|| format!("{error}"), |code| code.into_owned())
}

fn run<F>(body: F) -> TestResult
where
    F: AsyncFnOnce(String) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(body(admin))
}

/// A raw ring record of `item_seed` at `expected`, cause `cause`, with the given full values and
/// before and after values (SQL literals); the row write follows in the caller's script.
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
        let timed = Timed::create(admin, "first").await?;
        assert_eq!(timed.state(RING).await?, None);

        let first = ring(0, 1, RING_MS - 59_000)?;
        assert_eq!(
            timed.checkpoint(1, &first).await.map_err(debug)?,
            TimedWriteOutcome::Written { revision: 1 }
        );
        let stored = timed.state(RING).await?.ok_or("no row")?;
        assert_eq!(stored.revision, 1);
        assert_eq!(stored.values, Some(values(None, Some(RING_MS - 59_000))?));
        assert_eq!(stored.deadline_at, None);
        // A checkpoint never advances the CharacterRevision.
        assert_eq!(timed.harness.root_revision().await?, "1");

        let mut connection = timed.harness.pool.acquire().await?;
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
        assert_eq!(
            timed.checkpoint(1, &first).await.map_err(debug)?,
            TimedWriteOutcome::AlreadyCommitted { revision: 1 }
        );
        // A second insert at expected revision 0 writes nothing.
        assert!(matches!(
            timed.checkpoint(1, &ring(0, 2, RING_MS - 60_000)?).await,
            Err(TimedWriteError::ConflictingWrite)
        ));
        assert_eq!(timed.records().await?, 1);

        // The next checkpoint moves the row up by one from the stored value.
        assert_eq!(
            timed
                .checkpoint(1, &ring(1, 3, RING_MS - 119_000)?)
                .await
                .map_err(debug)?,
            TimedWriteOutcome::Written { revision: 2 }
        );
        assert_eq!(timed.state(RING).await?.ok_or("no row")?.revision, 2);
        timed.cleanup().await
    })
}

#[test]
fn a_stale_fence_writes_nothing() -> TestResult {
    run(async |admin| {
        let timed = Timed::create(admin, "fence").await?;
        let request = ring(0, 1, RING_MS - 1_000)?;
        // A stale CharacterRevision.
        assert!(matches!(
            timed.checkpoint(2, &request).await,
            Err(TimedWriteError::CharacterRevisionMismatch)
        ));
        // A stale scope ownership generation.
        let mut stale_scope = fence(1)?;
        stale_scope.scope_ownership_generation =
            crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?;
        let seal = timed.harness.recovery.seal_current().map_err(debug)?;
        let authority = timed
            .harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(matches!(
            timed
                .harness
                .root
                .commit_timed_checkpoint(
                    &authority,
                    &timed.harness.node,
                    stale_scope,
                    request.clone()
                )
                .await,
            Err(TimedWriteError::AuthorityRejected)
        ));
        drop(authority);
        drop(seal);
        // An ended session.
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions SET session_state = 3 \
             WHERE game_session_id = encode($1,'hex')::uuid",
        )
        .bind(id(SESSION).as_slice())
        .execute(&timed.harness.pool)
        .await?;
        assert!(matches!(
            timed.checkpoint(1, &request).await,
            Err(TimedWriteError::AuthorityRejected)
        ));
        assert_eq!(timed.records().await?, 0);
        assert_eq!(timed.state(RING).await?, None);
        timed.cleanup().await
    })
}

#[test]
fn refused_checkpoints_write_nothing() -> TestResult {
    run(async |admin| {
        let timed = Timed::create(admin, "refuse").await?;
        timed
            .checkpoint(1, &ring(0, 1, RING_MS - 1_000)?)
            .await
            .map_err(debug)?;
        // Unchanged values write nothing; a value above the row never adds time.
        for remaining in [RING_MS - 1_000, RING_MS - 500] {
            assert!(matches!(
                timed.checkpoint(1, &ring(1, 2, remaining)?).await,
                Err(TimedWriteError::NotAStoreableChange)
            ));
        }
        // A stale or future expected revision finds another revision.
        assert!(matches!(
            timed.checkpoint(1, &ring(5, 3, RING_MS - 2_000)?).await,
            Err(TimedWriteError::RevisionMismatch {
                current: Some(StoredTimedState { revision: 1, .. }),
            })
        ));
        // The fenced Character holds neither an unplaced ring nor a Ground item.
        for seed in [OTHER_RING, GROUND_RING] {
            let mut foreign = ring(0, 4, RING_MS - 2_000)?;
            foreign.item_instance_id = id(seed);
            assert!(matches!(
                timed.checkpoint(1, &foreign).await,
                Err(TimedWriteError::NotHeld)
            ));
        }
        // Values outside the definition are invalid input.
        let mut wrong_shape = ring(1, 6, RING_MS - 2_000)?;
        wrong_shape.values = values(Some(3), Some(RING_MS - 2_000))?;
        assert!(matches!(
            timed.checkpoint(1, &wrong_shape).await,
            Err(TimedWriteError::InvalidInput)
        ));
        assert_eq!(timed.records().await?, 1);
        assert_eq!(timed.state(RING).await?.ok_or("no row")?.revision, 1);
        timed.cleanup().await
    })
}

#[test]
fn a_charges_checkpoint_stores_the_live_charges() -> TestResult {
    run(async |admin| {
        let timed = Timed::create(admin, "charges").await?;
        let request = TimedCheckpointRequest {
            item_instance_id: id(AMULET),
            expected_revision: 0,
            transaction_id: id(1),
            definition: facts(AMULET_KEY, values(Some(AMULET_CHARGES), None)?, false),
            values: values(Some(AMULET_CHARGES - 7), None)?,
        };
        assert_eq!(
            timed.checkpoint(1, &request).await.map_err(debug)?,
            TimedWriteOutcome::Written { revision: 1 }
        );
        assert_eq!(
            timed.state(AMULET).await?.ok_or("no row")?.values,
            Some(values(Some(AMULET_CHARGES - 7), None)?)
        );
        timed.cleanup().await
    })
}

#[test]
fn the_guard_refuses_a_lit_item_in_a_container_and_other_full_values() -> TestResult {
    run(async |admin| {
        let timed = Timed::create(admin, "lit").await?;
        // A lit torch in a backpack entry: the guard refuses it at commit (D360).
        let torch = TimedCheckpointRequest {
            item_instance_id: id(TORCH),
            expected_revision: 0,
            transaction_id: id(1),
            definition: facts(TORCH_KEY, values(None, Some(600_000))?, true),
            values: values(None, Some(500_000))?,
        };
        assert!(matches!(
            timed.checkpoint(1, &torch).await,
            Err(TimedWriteError::Unavailable(_))
        ));
        // The first write pins a definition's full values; a later one claiming others is
        // refused.
        timed
            .checkpoint(1, &ring(0, 2, RING_MS - 1_000)?)
            .await
            .map_err(debug)?;
        let mut inflated = ring(1, 3, RING_MS - 2_000)?;
        inflated.definition.full = values(None, Some(RING_MS * 2))?;
        assert!(matches!(
            timed.checkpoint(1, &inflated).await,
            Err(TimedWriteError::Unavailable(_))
        ));
        assert_eq!(timed.records().await?, 1);
        assert_eq!(timed.state(TORCH).await?, None);
        timed.cleanup().await
    })
}

#[test]
fn the_guard_refuses_writes_without_their_record_or_row() -> TestResult {
    run(async |admin| {
        let timed = Timed::create(admin, "guard").await?;
        let full_ms = RING_MS.to_string();
        let full = ("NULL", full_ms.as_str(), false);
        let before = ("NULL", full_ms.as_str());
        let row = |revision: u8, deadline: &str| {
            format!(
                "INSERT INTO game_item_timed_states VALUES ({}, NULL, 1000, {deadline}, {revision});",
                uuid(RING)
            )
        };
        let refusals = [
            (row(1, "NULL"), "row without record"),
            (
                raw_record(RING, 0, 1, 1, full, before, ("NULL", "1000", "NULL")),
                "record without row",
            ),
            (
                format!(
                    "{}{}",
                    raw_record(RING, 0, 2, 2, full, before, ("NULL", "1000", "NULL")),
                    row(1, "NULL")
                ),
                "unadmitted cause (Expire)",
            ),
            (
                format!(
                    "{}{}",
                    raw_record(RING, 0, 1, 3, full, before, ("NULL", "1000", "99999999999")),
                    row(1, "99999999999")
                ),
                "deadline on a held item",
            ),
            (
                format!(
                    "{}{}",
                    raw_record(RING, 0, 1, 4, full, before, ("NULL", "1000", "NULL")),
                    row(2, "NULL")
                ),
                "row created at revision 2",
            ),
        ];
        for (script, label) in refusals {
            assert_eq!(
                timed.runtime_sql(script).await?.as_deref(),
                Some("23514"),
                "{label}"
            );
        }
        assert_eq!(timed.records().await?, 0);

        // The admitted shape commits through the raw SQL too.
        let admitted = format!(
            "{}{}",
            raw_record(RING, 0, 1, 5, full, before, ("NULL", "1000", "NULL")),
            row(1, "NULL")
        );
        assert_eq!(
            timed.runtime_sql(admitted).await?,
            None,
            "admitted checkpoint"
        );
        // A revision jump, a delete, a record change and a truncate are refused.
        for (script, label) in [
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
            assert!(
                timed.runtime_sql(script).await?.is_some(),
                "{label} must be refused"
            );
        }
        assert_eq!(timed.state(RING).await?.ok_or("no row")?.revision, 1);
        assert_eq!(timed.records().await?, 1);
        timed.cleanup().await
    })
}
