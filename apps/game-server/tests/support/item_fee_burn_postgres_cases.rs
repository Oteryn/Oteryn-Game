// Shared GOLD-FEE-1a cases (migration 0023). Any wrapper that provides the same path-loaded crate
// root as `item_fee_burn_postgres.rs` can include this file.
//
// The fee source of CHARM-6 does not exist yet, so the composed Character change is the exact SQL
// of a CHARM-3 unlock (root and state advanced by one with one charm receipt), issued in the same
// runtime-role transaction as `burn_fee_in_transaction`. The fixture (character, main backpack,
// coin stacks) is seeded by the migration owner with the guards off, as other item cases do.

use crate::domain::charm::CharmKey;
use crate::domain::{CharacterId, CharacterRevision};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::charm_state::CharmCommandOccurrence;
use crate::durability::item_fee_burn::{
    BurnedCoinStack, FeeBurnCause, FeeBurnError, FeeBurnOutcome, FeeBurnRequest,
    burn_fee_in_transaction,
};
use crate::durability::item_fee_burn_audit::decode_fee_burn_envelope;
use crate::foundation::{
    ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    WorldId,
};
use sqlx::{Connection, Executor, PgPool};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CHARACTER: u8 = 41;
const ACCOUNT: u8 = 44;
const WORLD: u8 = 42;
const CHANNEL: u8 = 43;
const BACKPACK: u8 = 50;
const GOLD: &str = "oteryn:item.tibia.i3031";
const PLATINUM: &str = "oteryn:item.tibia.i3035";
const OCCURRED_AT: i64 = 1_790_000_000_000;

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
    /// A login in the runtime group (0006): every fee transaction runs as it.
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
        let name = format!("fee_{tag}_{suffix}");
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

        let role = format!("fee_runtime_{tag}_{suffix}");
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
                   'profile-1', 'ruleset-1', 'content-1', 'starter-1'); \
                 INSERT INTO game_character_progression_state VALUES ({character}, 1, 50, 1000, \
                   'profile-1', 'ruleset-1', 'content-1', 'simulation-1', 'evidence-1', \
                   'declaration-1', 'policy-1', 'reward-1'); \
                 {backpack} \
                 INSERT INTO game_item_container_slots(character_id, item_instance_id, world_id, \
                   placed_transaction_id) VALUES ({character}, {backpack_id}, {world}, {placed});",
                character = uuid(CHARACTER),
                account = uuid(ACCOUNT),
                world = uuid(WORLD),
                backpack = item(BACKPACK, "oteryn:item.tibia.i2854", "rev-1", 1),
                backpack_id = uuid(BACKPACK),
                placed = uuid(BACKPACK + 1),
            ))
            .await?;
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

    /// A live stack of `key` in a new direct backpack entry at `ordinal`. Item `seed` uses the
    /// ids `seed` (item), `seed + 1` (minted) and `seed + 2` (placed).
    async fn stack(&self, seed: u8, key: &str, quantity: u32, ordinal: u64) -> TestResult {
        self.stack_at_revision(seed, key, quantity, ordinal, "rev-1")
            .await
    }

    async fn stack_at_revision(
        &self,
        seed: u8,
        key: &str,
        quantity: u32,
        ordinal: u64,
        revision: &str,
    ) -> TestResult {
        self.seed(&format!(
            "{} INSERT INTO game_item_container_entries(item_instance_id, world_id, character_id, \
               parent_item_instance_id, placement_ordinal, placed_transaction_id) \
             VALUES ({}, {}, {}, {}, {ordinal}, {});",
            item(seed, key, revision, quantity),
            uuid(seed),
            uuid(WORLD),
            uuid(CHARACTER),
            uuid(BACKPACK),
            uuid(seed + 2),
        ))
        .await
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

fn item(seed: u8, key: &str, revision: &str, quantity: u32) -> String {
    format!(
        "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
           definition_production_key, definition_revision_ref, quantity, lifecycle, \
           minted_transaction_id) \
         VALUES ({}, {}, 'Item', '{key}', '{revision}', {quantity}, 1, {});",
        uuid(seed),
        uuid(WORLD),
        uuid(seed + 1),
    )
}

fn fence(revision: u64) -> TestResult<CurrentCharacterGameplayFence> {
    Ok(CurrentCharacterGameplayFence {
        character_id: CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?,
        game_session_id: GameSessionId::decode(&id(45)).map_err(debug)?,
        connection_generation: ConnectionGeneration::new(1).map_err(debug)?,
        character_lease_generation: 1,
        runtime_scope: RuntimeScopeRefV1::channel(
            WorldId::decode(&id(WORLD)).map_err(debug)?,
            ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
        ),
        scope_ownership_generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
        expected_character_revision: CharacterRevision::new(revision).map_err(debug)?,
    })
}

/// A `CharmUnassign` fee of occurrence `occurrence` with TransactionId `occurrence + 100`.
fn request(occurrence: u8, fee: u64) -> TestResult<FeeBurnRequest> {
    Ok(FeeBurnRequest {
        cause: FeeBurnCause::CharmUnassign {
            charm: CharmKey::new(format!("oteryn:charm.c{occurrence}")).map_err(debug)?,
            occurrence: CharmCommandOccurrence::from_bytes(id(occurrence)).map_err(debug)?,
        },
        fee_gold_units: fee,
        transaction_id: id(occurrence + 100),
        event_id: id(occurrence + 130),
        occurred_at_unix_ms: OCCURRED_AT,
        server_build_id: "fee-test-build".into(),
    })
}

/// The Character change of the fee source: the exact SQL of a CHARM-3 unlock of charm
/// `c<occurrence>` at `original`.
fn character_change(occurrence: u8, original: u64) -> String {
    let committed = original + 1;
    format!(
        "UPDATE game_character_roots SET character_revision = {committed} \
          WHERE character_id = {character} AND character_revision = {original}; \
         UPDATE game_character_progression_state SET character_revision = {committed} \
          WHERE character_id = {character} AND character_revision = {original}; \
         INSERT INTO game_character_charm_receipts(\
           charm_occurrence_id, command_binding, catalogue_digest, catalogue_revision, \
           character_id, original_character_revision, committed_character_revision, \
           level_before, level_after, experience_before, experience_after, command_kind, \
           charm_key, charm_category, stage_before, stage_after, stage_cost, race_key, \
           profile_revision, ruleset_revision, content_revision, simulation_revision, \
           evidence_revision, declaration_revision, policy_revision, reward_revision, \
           committed_at) \
         VALUES ({occurrence_id}, '\\x{binding}'::bytea, '\\x{digest}'::bytea, 'content-1', \
           {character}, {original}, {committed}, 50, 50, 1000, 1000, 1, \
           'oteryn:charm.c{occurrence}', 1, 0, 1, 240, NULL, \
           'profile-1', 'ruleset-1', 'content-1', 'simulation-1', 'evidence-1', \
           'declaration-1', 'policy-1', 'reward-1', 1); \
         INSERT INTO game_character_charm_unlocks VALUES \
           ({character}, 'oteryn:charm.c{occurrence}', 1, {committed}, {occurrence_id});",
        character = uuid(CHARACTER),
        occurrence_id = uuid(occurrence),
        binding = hex(&[occurrence; 33]),
        digest = hex(&[occurrence; 32]),
    )
}

#[derive(Debug)]
enum Composed {
    Committed(FeeBurnOutcome),
    Refused(FeeBurnError),
    /// The SQLSTATE of a refused commit.
    CommitFailed(String),
}

/// One fee source transaction as the runtime role: optionally its Character change, then the
/// burn, then commit (or rollback on a refusal).
async fn compose(
    harness: &Harness,
    original: u64,
    request: &FeeBurnRequest,
    with_change: bool,
) -> TestResult<Composed> {
    let FeeBurnCause::CharmUnassign { occurrence, .. } = &request.cause;
    let mut tx = harness.runtime.begin().await?;
    if with_change {
        sqlx::raw_sql(sqlx::AssertSqlSafe(character_change(
            occurrence.as_bytes()[0],
            original,
        )))
        .execute(&mut *tx)
        .await?;
    }
    match burn_fee_in_transaction(&mut tx, &fence(original)?, request).await {
        Ok(outcome) => match tx.commit().await {
            Ok(()) => Ok(Composed::Committed(outcome)),
            Err(error) => Ok(Composed::CommitFailed(sqlstate(&error))),
        },
        Err(error) => {
            tx.rollback().await?;
            Ok(Composed::Refused(error))
        }
    }
}

fn sqlstate(error: &sqlx::Error) -> String {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .map_or_else(|| format!("{error}"), |code| code.into_owned())
}

/// Every durable fact a fee transaction may change.
async fn snapshot(pool: &PgPool) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT concat_ws('|', \
           (SELECT character_revision::text FROM game_character_roots), \
           (SELECT string_agg(concat_ws(':', item_instance_id, quantity, lifecycle, \
                     last_transaction_id), ',' ORDER BY item_instance_id) \
              FROM game_item_instances), \
           (SELECT string_agg(concat_ws(':', item_instance_id, placement_ordinal), ',' \
                     ORDER BY placement_ordinal) FROM game_item_container_entries), \
           (SELECT count(*) FROM game_item_fee_burns), \
           (SELECT count(*) FROM game_item_fee_burn_lines), \
           (SELECT count(*) FROM game_item_audit_outbox), \
           (SELECT count(*) FROM game_character_charm_receipts))",
    )
    .fetch_one(pool)
    .await?)
}

async fn quantities(pool: &PgPool) -> TestResult<Vec<(i64, i16, bool)>> {
    // (quantity, lifecycle, has an entry) of every coin, by item seed.
    Ok(sqlx::query_as(
        "SELECT i.quantity, i.lifecycle, \
                EXISTS (SELECT 1 FROM game_item_container_entries e \
                         WHERE e.item_instance_id = i.item_instance_id) \
           FROM game_item_instances i \
          WHERE i.definition_production_key <> 'oteryn:item.tibia.i2854' \
          ORDER BY i.item_instance_id",
    )
    .fetch_all(pool)
    .await?)
}

fn line(seed: u8, ordinal: u64, before: u32, after: u32) -> BurnedCoinStack {
    BurnedCoinStack {
        item_instance_id: id(seed),
        placement_ordinal: ordinal,
        quantity_before: before,
        quantity_after: after,
    }
}

#[test]
fn a_fee_burns_whole_stacks_then_part_of_the_last_with_the_character_change() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "burn").await?;
        harness.stack(100, GOLD, 30, 1).await?;
        harness.stack(110, GOLD, 50, 2).await?;
        harness.stack(120, GOLD, 40, 3).await?;
        // Not admitted in GOLD-FEE-1a, and not a coin: both stay untouched.
        harness.stack(130, PLATINUM, 5, 4).await?;
        harness.stack(140, "oteryn:item.tibia.i2853", 1, 5).await?;

        let request = request(61, 100)?;
        let Composed::Committed(FeeBurnOutcome::Burned(burned)) =
            compose(&harness, 1, &request, true).await?
        else {
            return Err("the fee did not commit".into());
        };
        // Display order (highest ordinal first); only the last stack keeps units.
        let expected = vec![
            line(120, 3, 40, 0),
            line(110, 2, 50, 0),
            line(100, 1, 30, 20),
        ];
        assert_eq!(burned.lines, expected);
        assert_eq!(burned.committed_character_revision.get(), 2);
        assert_eq!(
            quantities(&harness.pool).await?,
            vec![
                (20, 1, true),
                (0, 2, false),
                (0, 2, false),
                (5, 1, true),
                (1, 1, true)
            ]
        );
        let (revision, fee, burned_units, lines): (String, i64, i64, i16) = sqlx::query_as(
            "SELECT f.committed_character_revision::text, f.fee_gold_units, \
                    f.burned_gold_units, f.line_count \
               FROM game_item_fee_burns f \
               JOIN game_character_roots r ON r.character_id = f.character_id \
              WHERE r.character_revision = f.committed_character_revision",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(
            (revision.as_str(), fee, burned_units, lines),
            ("2", 100, 100, 3)
        );

        // The one audit event decodes through the registered fee gates.
        let envelope: Vec<u8> = sqlx::query_scalar(
            "SELECT envelope FROM game_item_audit_outbox \
              WHERE event_id = encode($1,'hex')::uuid",
        )
        .bind(request.event_id.as_slice())
        .fetch_one(&harness.pool)
        .await?;
        let (_, event) = decode_fee_burn_envelope(&envelope).map_err(debug)?;
        assert_eq!(event.fee_gold_units, 100);
        assert_eq!(event.lines.len(), 3);
        assert_eq!(event.committed_character_revision, 2);

        // Exact occurrence replay returns the first outcome and writes nothing.
        let before = snapshot(&harness.pool).await?;
        let mut tx = harness.runtime.begin().await?;
        let replay = burn_fee_in_transaction(&mut tx, &fence(1)?, &request).await;
        tx.rollback().await?;
        match replay {
            Ok(FeeBurnOutcome::AlreadyBurned(first)) => assert_eq!(first, burned),
            other => return Err(format!("expected the retained outcome, got {other:?}").into()),
        }
        // A changed binding for the same occurrence conflicts.
        let mut changed = request.clone();
        changed.fee_gold_units = 101;
        let mut tx = harness.runtime.begin().await?;
        let conflict = burn_fee_in_transaction(&mut tx, &fence(1)?, &changed).await;
        tx.rollback().await?;
        if !matches!(conflict, Err(FeeBurnError::ConflictingOccurrence)) {
            return Err(format!("expected a conflict, got {conflict:?}").into());
        }
        assert_eq!(snapshot(&harness.pool).await?, before);
        harness.cleanup().await
    })
}

/// (case, request, fence revision, expected refusal).
type RefusalCase = (&'static str, FeeBurnRequest, u64, fn(&FeeBurnError) -> bool);

#[test]
fn refusals_leave_the_source_transaction_to_roll_back_and_write_nothing() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "refuse").await?;
        harness.stack(100, GOLD, 30, 1).await?;
        harness.stack(110, PLATINUM, 5, 2).await?;
        let before = snapshot(&harness.pool).await?;

        // 31 gold units exceed the gold; platinum pays only from GOLD-FEE-1b on.
        let cases: [RefusalCase; 4] = [
            ("insufficient gold", request(61, 31)?, 1, |error| {
                matches!(error, FeeBurnError::InsufficientFunds)
            }),
            ("zero fee", request(62, 0)?, 1, |error| {
                matches!(error, FeeBurnError::InvalidInput)
            }),
            (
                "fee above the reachable maximum",
                request(63, 20_000_001)?,
                1,
                |error| matches!(error, FeeBurnError::InvalidInput),
            ),
            // The fence expects revision 5; the root is at 1 (2 after the change).
            ("stale Character revision", request(64, 10)?, 5, |error| {
                matches!(error, FeeBurnError::CharacterMismatch)
            }),
        ];
        for (case, request, revision, expected) in cases {
            let mut tx = harness.runtime.begin().await?;
            sqlx::raw_sql(sqlx::AssertSqlSafe(character_change(
                request.transaction_id[0] - 100,
                1,
            )))
            .execute(&mut *tx)
            .await?;
            let result = burn_fee_in_transaction(&mut tx, &fence(revision)?, &request).await;
            tx.rollback().await?;
            match &result {
                Err(error) if expected(error) => {}
                other => return Err(format!("{case}: unexpected {other:?}").into()),
            }
            assert_eq!(snapshot(&harness.pool).await?, before, "{case}");
        }

        // The whole gold pays exactly; the platinum stays.
        match compose(&harness, 1, &request(65, 30)?, true).await? {
            Composed::Committed(FeeBurnOutcome::Burned(burned)) => {
                assert_eq!(burned.lines, vec![line(100, 1, 30, 0)]);
            }
            other => return Err(format!("expected a commit, got {other:?}").into()),
        }
        assert_eq!(
            quantities(&harness.pool).await?,
            vec![(0, 2, false), (5, 1, true)]
        );
        harness.cleanup().await
    })
}

#[test]
fn the_database_refuses_a_burn_without_its_character_change() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "alone").await?;
        harness.stack(100, GOLD, 30, 1).await?;
        let before = snapshot(&harness.pool).await?;
        match compose(&harness, 1, &request(61, 10)?, false).await? {
            Composed::CommitFailed(code) if code == "23514" => {}
            other => return Err(format!("expected a refused commit, got {other:?}").into()),
        }
        assert_eq!(snapshot(&harness.pool).await?, before);

        // A committed fee leaves the root at revision 2. A later burn bound to that same revision
        // by a stale fence, with no Character change of its own, is refused: the root must be
        // written by the burn's own physical transaction.
        match compose(&harness, 1, &request(62, 10)?, true).await? {
            Composed::Committed(FeeBurnOutcome::Burned(_)) => {}
            other => return Err(format!("expected a commit, got {other:?}").into()),
        }
        let before = snapshot(&harness.pool).await?;
        match compose(&harness, 1, &request(63, 5)?, false).await? {
            Composed::CommitFailed(code) if code == "23514" => {}
            other => return Err(format!("expected a refused commit, got {other:?}").into()),
        }
        assert_eq!(snapshot(&harness.pool).await?, before);
        harness.cleanup().await
    })
}

#[test]
fn twenty_whole_burns_at_the_longest_revision_commit_one_fee_sized_event() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "twenty").await?;
        let revision = "r".repeat(512);
        for n in 0..20_u8 {
            harness
                .stack_at_revision(100 + 3 * n, GOLD, 100, u64::from(n) + 1, &revision)
                .await?;
        }
        let before = snapshot(&harness.pool).await?;
        match compose(&harness, 1, &request(61, 2_001)?, true).await? {
            Composed::Refused(FeeBurnError::InsufficientFunds) => {}
            other => return Err(format!("expected insufficient funds, got {other:?}").into()),
        }
        assert_eq!(snapshot(&harness.pool).await?, before);

        let request = request(62, 2_000)?;
        match compose(&harness, 1, &request, true).await? {
            Composed::Committed(FeeBurnOutcome::Burned(burned)) => {
                assert_eq!(burned.lines.len(), 20);
                assert!(burned.lines.iter().all(|line| line.quantity_after == 0));
            }
            other => return Err(format!("expected a commit, got {other:?}").into()),
        }
        let (entries, envelope): (i64, i32) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_item_container_entries), \
                    (SELECT octet_length(envelope) FROM game_item_audit_outbox \
                      WHERE event_id = encode($1,'hex')::uuid)",
        )
        .bind(request.event_id.as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(entries, 0);
        // Above the one-item 9,216 B, within the fee row.
        assert!(envelope > 9_216 && envelope <= 24_495, "{envelope}");
        harness.cleanup().await
    })
}

/// A hand-written fee transaction of one line; `claimed_before` is the line's quantity before.
#[allow(clippy::too_many_arguments)]
fn sql_fee(
    occurrence: u8,
    item: u8,
    ordinal: u64,
    claimed_before: u32,
    after: u32,
    fee: u32,
    with_change: bool,
    delete_entry: bool,
) -> String {
    let transaction = uuid(occurrence + 100);
    let event = uuid(occurrence + 130);
    format!(
        "{change}\
         INSERT INTO game_item_fee_burns(transaction_id, event_id, cause_kind, \
           cause_occurrence_id, charm_key, request_binding, character_id, world_id, channel_id, \
           runtime_scope_ownership_generation, committed_character_revision, \
           backpack_item_instance_id, fee_gold_units, burned_gold_units, change_gold_units, \
           line_count, occurred_at, envelope_sha256, committed_at) \
         VALUES ({transaction}, {event}, 1, {occurrence_id}, 'oteryn:charm.c{occurrence}', \
           '\\x{binding}'::bytea, {character}, {world}, {channel}, 1, 2, {backpack}, {fee}, \
           {fee}, 0, 1, {OCCURRED_AT}, sha256('\\x0102'::bytea), 1); \
         INSERT INTO game_item_fee_burn_lines(transaction_id, line_ordinal, item_instance_id, \
           placement_ordinal, coin_worth, quantity_before, quantity_after) \
         VALUES ({transaction}, 1, {item_id}, {ordinal}, 1, {claimed_before}, {after}); \
         UPDATE game_item_instances SET quantity = {after}, lifecycle = {lifecycle}, \
           last_transaction_id = {transaction} WHERE item_instance_id = {item_id}; \
         {delete} \
         INSERT INTO game_item_audit_outbox(event_id, transaction_id, transaction_ordinal, \
           transaction_count, event_type_id, schema_revision, retention_profile_id, \
           item_instance_id, occurred_at, expires_at, envelope, envelope_sha256, \
           publication_state) \
         VALUES ({event}, {transaction}, 1, 1, 2, 1, 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1', \
           {item_id}, {OCCURRED_AT}, {OCCURRED_AT} + 7776000000, '\\x0102'::bytea, \
           sha256('\\x0102'::bytea), 1);",
        change = if with_change {
            character_change(occurrence, 1)
        } else {
            String::new()
        },
        occurrence_id = uuid(occurrence),
        binding = hex(&[occurrence; 32]),
        character = uuid(CHARACTER),
        world = uuid(WORLD),
        channel = uuid(CHANNEL),
        backpack = uuid(BACKPACK),
        item_id = uuid(item),
        lifecycle = if after == 0 { 2 } else { 1 },
        delete = if delete_entry {
            format!(
                "DELETE FROM game_item_container_entries WHERE item_instance_id = {};",
                uuid(item)
            )
        } else {
            String::new()
        },
    )
}

async fn expect_rejected(harness: &Harness, case: &str, script: &str) -> TestResult {
    let before = snapshot(&harness.pool).await?;
    let mut tx = harness.runtime.begin().await?;
    let result = match sqlx::raw_sql(sqlx::AssertSqlSafe(script.to_owned()))
        .execute(&mut *tx)
        .await
    {
        Ok(_) => tx.commit().await,
        Err(error) => Err(error),
    };
    match &result {
        Err(error) if sqlstate(error) == "23514" => {}
        other => return Err(format!("{case}: expected SQLSTATE 23514, got {other:?}").into()),
    }
    if snapshot(&harness.pool).await? != before {
        return Err(format!("{case}: a refused transaction changed durable state").into());
    }
    Ok(())
}

/// The exact statements a writer issues, as the runtime role, with one invariant broken per case.
#[test]
fn the_database_binds_every_burn_to_its_plan_evidence_and_character_change() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "guards").await?;
        harness.stack(100, GOLD, 30, 1).await?;
        harness.stack(110, GOLD, 50, 2).await?;
        harness.stack(120, PLATINUM, 5, 3).await?;
        let entry = |seed: u8| {
            format!(
                "DELETE FROM game_item_container_entries WHERE item_instance_id = {};",
                uuid(seed)
            )
        };
        expect_rejected(&harness, "an entry removed without a fee line", &entry(100)).await?;
        expect_rejected(
            &harness,
            "an oversize audit event without a fee record",
            &format!(
                "INSERT INTO game_item_audit_outbox(event_id, transaction_id, \
                   transaction_ordinal, transaction_count, event_type_id, schema_revision, \
                   retention_profile_id, item_instance_id, occurred_at, expires_at, envelope, \
                   envelope_sha256, publication_state) \
                 VALUES ({}, {}, 1, 1, 2, 1, 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1', {}, \
                   1, 7776000001, decode(repeat('00', 9217), 'hex'), \
                   sha256(decode(repeat('00', 9217), 'hex')), 1);",
                uuid(200),
                uuid(201),
                uuid(100)
            ),
        )
        .await?;
        let cases = [
            (
                "a later stack burned before the first in display order",
                sql_fee(61, 100, 1, 30, 0, 30, true, true),
            ),
            (
                "a false before quantity",
                sql_fee(62, 110, 2, 45, 20, 25, true, false),
            ),
            (
                "no Character change",
                sql_fee(63, 110, 2, 50, 20, 30, false, false),
            ),
            (
                "a partial burn removes its entry",
                sql_fee(64, 110, 2, 50, 20, 30, true, true),
            ),
            (
                "a whole burn keeps its entry",
                sql_fee(65, 110, 2, 50, 0, 50, true, false),
            ),
            (
                "a whole burn at another ordinal",
                sql_fee(66, 110, 7, 50, 0, 50, true, true),
            ),
            (
                "platinum burned as gold",
                sql_fee(67, 120, 3, 5, 0, 5, true, true),
            ),
        ];
        for (case, script) in &cases {
            expect_rejected(&harness, case, script).await?;
        }
        // The same statements with every invariant kept commit.
        let mut tx = harness.runtime.begin().await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql_fee(
            68, 110, 2, 50, 20, 30, true, false,
        )))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        assert_eq!(
            quantities(&harness.pool).await?,
            vec![(30, 1, true), (20, 1, true), (5, 1, true)]
        );
        // Fee records and lines are immutable, even for the migration owner; the runtime role
        // has no UPDATE or DELETE grant on them at all.
        for statement in [
            "UPDATE game_item_fee_burns SET fee_gold_units = 1",
            "DELETE FROM game_item_fee_burn_lines",
        ] {
            let owner = sqlx::raw_sql(sqlx::AssertSqlSafe(statement))
                .execute(&harness.pool)
                .await;
            assert_eq!(owner.as_ref().err().map(sqlstate).as_deref(), Some("23514"));
            let runtime = sqlx::raw_sql(sqlx::AssertSqlSafe(statement))
                .execute(&harness.runtime)
                .await;
            assert_eq!(
                runtime.as_ref().err().map(sqlstate).as_deref(),
                Some("42501")
            );
        }
        harness.cleanup().await
    })
}
