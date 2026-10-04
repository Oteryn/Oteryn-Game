#![allow(clippy::expect_used, clippy::unwrap_used)]
// Shared GOLD-FEE-2 cases (migration 0072): the bank part of a gold fee (BANK-FEE-0 §3-§5) and the
// type-2 `(1, V1)` / `(2, V2)` readiness of ARCH-BATCH-ROOT-PACKETS-V1 §1.7 phase 1, run by the
// CI-run `character_authority_postgres` target. Ordinary workspace runs report
// PRE-ROUTING/NONCANONICAL when the routed database is absent.
//
// The fee source of CHARM-6 does not exist yet, so the composed Character change is the exact SQL
// of a CHARM-3 unlock, issued in the same runtime-role transaction as the fee writer (as the
// GOLD-FEE-1a/1b cases do). The bank path is driven with `(2, V2)` explicitly; production code
// emits `(1, V1)` in phase 1.

use crate::domain::charm::CharmKey;
use crate::domain::currency::Coin;
use crate::domain::{CharacterId, CharacterRevision};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::charm_state::CharmCommandOccurrence;
use crate::durability::item_fee_burn::{
    BurnedCoinStack, CommittedFeeBurn, FeeBurnCause, FeeBurnError, FeeBurnOutcome, FeeBurnRequest,
    FeeChangeFacts, burn_fee_in_transaction, burn_fee_in_transaction_under,
};
use crate::durability::item_fee_burn_audit::{
    FEE_RL03_VALUE_LINES_MAX, FEE_RL07_ENVELOPE_BYTES_MAX, FeeDebitEntryFacts,
    check_bank_debit_matches_entry, decode_fee_burn_envelope, fee_burn_usage,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_mint_audit::golden::{TYPE2_GOLDEN_V1, unhex, verify_type2_shape};
use crate::durability::item_mint_audit::{EventEnvelopeV1, Type2EventTuple};
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};
use crate::foundation::{
    ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    WorldId,
};
use prost::Message;
use sqlx::{Connection, Executor, PgPool};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CHARACTER: u8 = 41;
const WORLD: u8 = 42;
const CHANNEL: u8 = 43;
const ACCOUNT: u8 = 44;
const OTHER_ACCOUNT: u8 = 46;
const OTHER_CHARACTER: u8 = 47;
const BACKPACK: u8 = 50;
const GOLD: &str = "oteryn:item.tibia.i3031";
const PLATINUM: &str = "oteryn:item.tibia.i3035";
const CRYSTAL: &str = "oteryn:item.tibia.i3043";
const BACKPACK_KEY: &str = "oteryn:item.tibia.i2854";
const OCCURRED_AT: i64 = 1_790_000_000_000;
const BALANCE_MAX: u64 = 999_999_999_999;
const V1: Type2EventTuple = Type2EventTuple::V1;
const V2: Type2EventTuple = Type2EventTuple::V2;

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

/// A second UUIDv7 family for ids that must not collide with `id`.
fn salted(seed: u8, salt: u8) -> [u8; 16] {
    let mut bytes = id(seed);
    bytes[1] = salt;
    bytes
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
    url: String,
    /// The migration owner, for fixtures and read-back only.
    pool: PgPool,
    /// A login in the runtime group (0006): every fee transaction runs as it.
    runtime: PgPool,
}

/// The payer and its world: with or without an equipped main backpack.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Payer {
    WithBackpack,
    WithoutBackpack,
}

impl Harness {
    /// A fresh database migrated through `upto` (every migration when `None`), the runtime login,
    /// and the payer root with its Account guard (and backpack).
    async fn create(
        admin_url: String,
        tag: &str,
        upto: Option<i64>,
        payer: Payer,
    ) -> TestResult<Self> {
        if !admin_url.starts_with("postgresql://oteryn_test_admin:")
            || !admin_url.ends_with("@127.0.0.1:5432/postgres")
        {
            return Err("unsafe PostgreSQL test admin URL".into());
        }
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = format!("feebank_{tag}_{suffix}");
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
        let migrator = sqlx::migrate!("./migrations");
        match upto {
            Some(version) => migrator.run_to(version, &mut connection).await?,
            None => migrator.run(&mut connection).await?,
        }
        connection.close().await?;
        let pool = PgPool::connect(&url).await?;

        let role = format!("feebank_runtime_{tag}_{suffix}");
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
            url,
            pool,
            runtime,
        };
        let backpack = match payer {
            Payer::WithBackpack => format!(
                "{} INSERT INTO game_item_container_slots(character_id, item_instance_id, \
                   world_id, placed_transaction_id) VALUES ({}, {}, {}, {});",
                item(BACKPACK, BACKPACK_KEY, "rev-1", 1),
                uuid(CHARACTER),
                uuid(BACKPACK),
                uuid(WORLD),
                uuid(BACKPACK + 1),
            ),
            Payer::WithoutBackpack => String::new(),
        };
        harness
            .seed(&format!(
                "INSERT INTO game_character_account_guards VALUES ({account}), ({other}); \
                 INSERT INTO game_character_roots VALUES \
                   ({character}, {account}, {world}, 1, 1, 'profile-1', 'ruleset-1', \
                    'content-1', 'starter-1', 'Fee Hero'), \
                   ({other_character}, {other}, {world}, 1, 1, 'profile-1', 'ruleset-1', \
                    'content-1', 'starter-1', 'Other Hero'); \
                 INSERT INTO game_character_progression_state VALUES ({character}, 1, 50, 1000, \
                   'profile-1', 'ruleset-1', 'content-1', 'simulation-1', 'evidence-1', \
                   'declaration-1', 'policy-1', 'reward-1'); \
                 {backpack}",
                character = uuid(CHARACTER),
                other_character = uuid(OTHER_CHARACTER),
                account = uuid(ACCOUNT),
                other = uuid(OTHER_ACCOUNT),
                world = uuid(WORLD),
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
        self.seed(&format!(
            "{} INSERT INTO game_item_container_entries(item_instance_id, world_id, character_id, \
               parent_item_instance_id, placement_ordinal, placed_transaction_id) \
             VALUES ({}, {}, {}, {}, {ordinal}, {});",
            item(seed, key, "rev-1", quantity),
            uuid(seed),
            uuid(WORLD),
            uuid(CHARACTER),
            uuid(BACKPACK),
            uuid(seed + 2),
        ))
        .await
    }

    /// A balance of `balance` on `account`, as one seeded deposit operation and its entry by
    /// `character` (the BANK-1 cases' fixture).
    async fn balance(&self, account: u8, character: u8, balance: u64) -> TestResult {
        let operation = format!("'{}'::uuid", hex(&salted(account, 9)));
        let entry = format!("'{}'::uuid", hex(&salted(account, 10)));
        self.seed(&format!(
            "INSERT INTO game_account_bank_operations(operation_occurrence_id, transaction_id, \
               event_id, kind, request_binding, acting_character_id, account_id, world_id, \
               channel_id, runtime_scope_ownership_generation, amount, outcome, occurred_at, \
               envelope_sha256, committed_at, planned_platinum_item_instance_id, \
               planned_gold_item_instance_id, backpack_item_instance_id) \
             VALUES ('{occ}'::uuid, {operation}, '{event}'::uuid, 1, sha256('seed'), {character}, \
               {account}, {world}, {channel}, 1, 1, 0, 1, sha256('seed'), 0, '{p}'::uuid, \
               '{g}'::uuid, {backpack}); \
             INSERT INTO game_account_bank_entries(entry_id, transaction_id, account_id, \
               world_id, kind, amount, balance_before, balance_after, acting_character_id) \
             VALUES ({entry}, {operation}, {account}, {world}, 1, {balance}, 0, {balance}, \
               {character}); \
             INSERT INTO game_account_bank_balances VALUES ({account}, {world}, {balance}, {entry});",
            occ = hex(&salted(account, 11)),
            event = hex(&salted(account, 12)),
            p = hex(&salted(account, 13)),
            g = hex(&salted(account, 14)),
            character = uuid(character),
            account = uuid(account),
            world = uuid(WORLD),
            channel = uuid(CHANNEL),
            backpack = uuid(BACKPACK),
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

fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "rev-1".into(),
    }
}

/// A `CharmUnassign` fee of occurrence `occurrence` with TransactionId `occurrence + 100`, EventId
/// `occurrence + 130` and change slots `occurrence + 150` (platinum) and `occurrence + 170` (gold).
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
        server_build_id: "fee-bank-test-build".into(),
        change: FeeChangeFacts {
            item_instance_ids: [id(occurrence + 150), id(occurrence + 170)],
            platinum: definition(PLATINUM),
            gold: definition(GOLD),
            crystal: definition(CRYSTAL),
            backpack: ItemDefinitionFacts {
                definition: definition(BACKPACK_KEY),
                stack: ItemStackClass::NonStackable,
                container_capacity: Some(20),
                container_slot_equip_pattern: true,
            },
        },
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
}

fn sqlstate(error: &sqlx::Error) -> String {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .map_or_else(|| format!("{error}"), |code| code.into_owned())
}

/// One fee source transaction as the runtime role: its Character change at `original`, then the
/// fee under `tuple`, then commit (or rollback on a refusal; `abort` rolls a success back too).
async fn compose_with(
    harness: &Harness,
    original: u64,
    request: &FeeBurnRequest,
    tuple: Type2EventTuple,
    abort: bool,
) -> TestResult<Composed> {
    let FeeBurnCause::CharmUnassign { occurrence, .. } = &request.cause;
    let mut tx = harness.runtime.begin().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(character_change(
        occurrence.as_bytes()[0],
        original,
    )))
    .execute(&mut *tx)
    .await?;
    let result = if tuple == V1 {
        // The production entry point.
        burn_fee_in_transaction(&mut tx, &fence(original)?, request).await
    } else {
        burn_fee_in_transaction_under(&mut tx, &fence(original)?, request, tuple).await
    };
    match result {
        Ok(outcome) if abort => {
            tx.rollback().await?;
            Ok(Composed::Committed(outcome))
        }
        Ok(outcome) => {
            tx.commit()
                .await
                .map_err(|error| format!("the commit was refused: {}", sqlstate(&error)))?;
            Ok(Composed::Committed(outcome))
        }
        Err(error) => {
            tx.rollback().await?;
            Ok(Composed::Refused(error))
        }
    }
}

async fn compose(
    harness: &Harness,
    original: u64,
    request: &FeeBurnRequest,
    tuple: Type2EventTuple,
) -> TestResult<Composed> {
    compose_with(harness, original, request, tuple, false).await
}

async fn pay(
    harness: &Harness,
    original: u64,
    request: &FeeBurnRequest,
    tuple: Type2EventTuple,
) -> TestResult<CommittedFeeBurn> {
    match compose(harness, original, request, tuple).await? {
        Composed::Committed(FeeBurnOutcome::Burned(burned)) => Ok(burned),
        other => Err(format!("expected a commit, got {other:?}").into()),
    }
}

/// Every durable fact a fee transaction may change, bank state included.
async fn snapshot(pool: &PgPool) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT concat_ws('|', \
           (SELECT character_revision::text FROM game_character_roots \
             WHERE character_id = $1::uuid), \
           (SELECT string_agg(concat_ws(':', item_instance_id, quantity, lifecycle, \
                     last_transaction_id), ',' ORDER BY item_instance_id) \
              FROM game_item_instances), \
           (SELECT string_agg(concat_ws(':', item_instance_id, placement_ordinal), ',' \
                     ORDER BY placement_ordinal) FROM game_item_container_entries), \
           (SELECT count(*) FROM game_item_fee_burns), \
           (SELECT count(*) FROM game_item_fee_burn_lines), \
           (SELECT count(*) FROM game_item_audit_outbox), \
           (SELECT count(*) FROM game_character_charm_receipts), \
           (SELECT count(*) FROM game_account_bank_entries), \
           (SELECT string_agg(concat_ws(':', account_id, balance, last_entry_id), ',' \
                     ORDER BY account_id) FROM game_account_bank_balances), \
           (SELECT count(*) FROM game_account_bank_audit_outbox))",
    )
    .bind(hex(&id(CHARACTER)))
    .fetch_one(pool)
    .await?)
}

async fn balance_of(pool: &PgPool, account: u8) -> TestResult<Option<i64>> {
    Ok(sqlx::query_scalar(
        "SELECT balance FROM game_account_bank_balances \
          WHERE account_id = $1::uuid AND world_id = $2::uuid",
    )
    .bind(hex(&id(account)))
    .bind(hex(&id(WORLD)))
    .fetch_optional(pool)
    .await?)
}

/// The stored event of a fee and its stored FEE_DEBIT entry, if any.
async fn stored_event(
    pool: &PgPool,
    request: &FeeBurnRequest,
) -> TestResult<(
    EventEnvelopeV1,
    crate::durability::item_fee_burn_audit::OneItemFeeBurnV1,
    (i64, String, Option<String>),
)> {
    let (envelope, schema_revision, profile, item): (Vec<u8>, i64, String, Option<String>) =
        sqlx::query_as(
            "SELECT envelope, schema_revision, retention_profile_id, item_instance_id::text \
               FROM game_item_audit_outbox WHERE event_id = $1::uuid",
        )
        .bind(hex(&request.event_id))
        .fetch_one(pool)
        .await?;
    let (decoded, burn) = decode_fee_burn_envelope(&envelope).map_err(debug)?;
    Ok((decoded, burn, (schema_revision, profile, item)))
}

async fn stored_entry(pool: &PgPool, request: &FeeBurnRequest) -> TestResult<FeeDebitEntryFacts> {
    let (entry, account, world, amount, before, after): (String, String, String, i64, i64, i64) =
        sqlx::query_as(
            "SELECT replace(entry_id::text, '-', ''), replace(account_id::text, '-', ''), \
                    replace(world_id::text, '-', ''), amount, balance_before, balance_after \
               FROM game_account_bank_entries \
              WHERE fee_transaction_id = $1::uuid AND kind = 5",
        )
        .bind(hex(&request.transaction_id))
        .fetch_one(pool)
        .await?;
    let bytes = |text: &str| -> TestResult<[u8; 16]> {
        unhex(text).try_into().map_err(|_| "not 16 bytes".into())
    };
    Ok(FeeDebitEntryFacts {
        entry_id: bytes(&entry)?,
        account_id: bytes(&account)?,
        world_id: bytes(&world)?,
        amount: u64::try_from(amount)?,
        balance_before: u64::try_from(before)?,
        balance_after: u64::try_from(after)?,
    })
}

fn whole(coin: Coin, seed: u8, ordinal: u64, quantity: u32) -> BurnedCoinStack {
    BurnedCoinStack {
        item_instance_id: id(seed),
        coin,
        placement_ordinal: ordinal,
        quantity_before: quantity,
        quantity_after: 0,
    }
}

#[test]
fn coins_first_then_the_bank_pays_the_rest_in_one_transaction() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "rest", None, Payer::WithBackpack).await?;
        harness.stack(100, GOLD, 30, 1).await?;
        harness.stack(110, PLATINUM, 5, 2).await?;
        // Not a coin: never touched.
        harness.stack(120, "oteryn:item.tibia.i2853", 1, 3).await?;
        harness.balance(ACCOUNT, CHARACTER, 10_000).await?;

        // T = 30 + 500 = 530 < F = 1,000: both stacks whole, no change, 470 from the bank.
        let request = request(61, 1_000)?;
        let burned = pay(&harness, 1, &request, V2).await?;
        assert_eq!(
            burned.lines,
            vec![
                whole(Coin::Gold, 100, 1, 30),
                whole(Coin::Platinum, 110, 2, 5)
            ]
        );
        assert!(burned.change.is_empty());
        assert_eq!(burned.change_gold_units, 0);
        let debit = burned.bank_debit.ok_or("no bank part")?;
        assert_eq!(
            (
                debit.account_id,
                debit.debit_gold_units,
                debit.balance_before_gold_units,
                debit.balance_after_gold_units
            ),
            (id(ACCOUNT), 470, 10_000, 9_530)
        );
        assert_eq!(balance_of(&harness.pool, ACCOUNT).await?, Some(9_530));

        // The record, its one FEE_DEBIT entry and the one fee event, which is (2, V2).
        let (bank, lines, burned_units): (i64, i16, i64) = sqlx::query_as(
            "SELECT bank_debit_gold_units, line_count, burned_gold_units FROM game_item_fee_burns",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!((bank, lines, burned_units), (470, 2, 530));
        let (envelope, event, (revision, profile, item)) =
            stored_event(&harness.pool, &request).await?;
        assert_eq!(
            (revision, profile.as_str()),
            (2, "DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2")
        );
        assert_eq!(envelope.event_schema_revision, 2);
        assert_eq!(item, Some(format!("{:x}", uuid_of(&id(100)))));
        let entry = stored_entry(&harness.pool, &request).await?;
        assert_eq!(entry.entry_id, debit.entry_id);
        check_bank_debit_matches_entry(&event, &entry).map_err(debug)?;
        assert_eq!(fee_burn_usage(&event).value_lines, FEE_RL03_VALUE_LINES_MAX);
        // A FEE_DEBIT emits no bank event.
        let bank_events: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_account_bank_audit_outbox")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(bank_events, 0);
        let untouched: (i64, i16) = sqlx::query_as(
            "SELECT quantity, lifecycle FROM game_item_instances WHERE item_instance_id = $1::uuid",
        )
        .bind(hex(&id(120)))
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(untouched, (1, 1));

        // The occurrence replay returns the first outcome, bank part included, and writes nothing.
        let before = snapshot(&harness.pool).await?;
        let mut tx = harness.runtime.begin().await?;
        let replay = burn_fee_in_transaction_under(&mut tx, &fence(1)?, &request, V2).await;
        tx.rollback().await?;
        match replay {
            Ok(FeeBurnOutcome::AlreadyBurned(first)) => assert_eq!(first, burned),
            other => return Err(format!("expected the retained outcome, got {other:?}").into()),
        }
        let mut changed = request.clone();
        changed.fee_gold_units = 1_001;
        let mut tx = harness.runtime.begin().await?;
        let conflict = burn_fee_in_transaction_under(&mut tx, &fence(1)?, &changed, V2).await;
        tx.rollback().await?;
        if !matches!(conflict, Err(FeeBurnError::ConflictingOccurrence)) {
            return Err(format!("expected a conflict, got {conflict:?}").into());
        }
        assert_eq!(snapshot(&harness.pool).await?, before);
        harness.cleanup().await
    })
}

/// The hyphenated text of a 16-byte id, as PostgreSQL prints a uuid.
fn uuid_of(bytes: &[u8; 16]) -> UuidText {
    UuidText(*bytes)
}

struct UuidText([u8; 16]);

impl std::fmt::LowerHex for UuidText {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = hex(&self.0);
        write!(
            formatter,
            "{}-{}-{}-{}-{}",
            &text[..8],
            &text[8..12],
            &text[12..16],
            &text[16..20],
            &text[20..]
        )
    }
}

#[test]
fn too_little_balance_and_phase_one_refuse_and_write_nothing() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "short", None, Payer::WithBackpack).await?;
        harness.stack(100, GOLD, 30, 1).await?;
        let before = snapshot(&harness.pool).await?;

        // No balance row at all: the zero row the balance step upserts is rolled back too.
        match compose(&harness, 1, &request(61, 100)?, V2).await? {
            Composed::Refused(FeeBurnError::InsufficientFunds) => {}
            other => return Err(format!("expected insufficient funds, got {other:?}").into()),
        }
        assert_eq!(snapshot(&harness.pool).await?, before);
        assert_eq!(balance_of(&harness.pool, ACCOUNT).await?, None);

        harness.balance(ACCOUNT, CHARACTER, 69).await?;
        let before = snapshot(&harness.pool).await?;
        // 70 is needed, 69 held.
        match compose(&harness, 1, &request(62, 100)?, V2).await? {
            Composed::Refused(FeeBurnError::InsufficientFunds) => {}
            other => return Err(format!("expected insufficient funds, got {other:?}").into()),
        }
        assert_eq!(snapshot(&harness.pool).await?, before);

        // Phase 1: production code emits (1, V1), so T < F stays refused as in stage 1 even when
        // the balance would cover it.
        match compose(&harness, 1, &request(63, 99)?, V1).await? {
            Composed::Refused(FeeBurnError::InsufficientFunds) => {}
            other => return Err(format!("expected insufficient funds, got {other:?}").into()),
        }
        // Above the coin part plus BANK0-RL-01: refused before any read under either tuple.
        for tuple in [V1, V2] {
            match compose(
                &harness,
                1,
                &request(64, 20_000_000 + BALANCE_MAX + 1)?,
                tuple,
            )
            .await?
            {
                Composed::Refused(FeeBurnError::InvalidInput) => {}
                other => return Err(format!("expected invalid input, got {other:?}").into()),
            }
        }
        assert_eq!(snapshot(&harness.pool).await?, before);

        // The same fee is paid when the coins cover it: (1, V1), no bank part, no ledger entry.
        let paid = pay(&harness, 1, &request(65, 30)?, V1).await?;
        assert_eq!(paid.bank_debit, None);
        let (_, _, (revision, profile, _)) = stored_event(&harness.pool, &request(65, 30)?).await?;
        assert_eq!(
            (revision, profile.as_str()),
            (1, "DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1")
        );
        assert_eq!(balance_of(&harness.pool, ACCOUNT).await?, Some(69));
        harness.cleanup().await
    })
}

#[test]
fn a_payer_without_coins_or_backpack_pays_wholly_from_the_bank() -> TestResult {
    run(async |admin| {
        // No backpack: T = 0.
        let harness = Harness::create(admin.clone(), "nobp", None, Payer::WithoutBackpack).await?;
        harness.balance(ACCOUNT, CHARACTER, 1_000).await?;
        let first = request(61, 1_000)?;
        let burned = pay(&harness, 1, &first, V2).await?;
        assert!(burned.lines.is_empty() && burned.change.is_empty());
        assert_eq!(
            burned
                .bank_debit
                .map(|debit| debit.balance_after_gold_units),
            Some(0)
        );
        let (backpack, lines, burned_units): (Option<String>, i16, i64) = sqlx::query_as(
            "SELECT backpack_item_instance_id::text, line_count, burned_gold_units \
               FROM game_item_fee_burns",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!((backpack, lines, burned_units), (None, 0, 0));
        let (_, event, (revision, _, item)) = stored_event(&harness.pool, &first).await?;
        assert_eq!((revision, item), (2, None));
        assert!(event.lines.is_empty() && event.backpack_item_instance_id.is_empty());
        check_bank_debit_matches_entry(&event, &stored_entry(&harness.pool, &first).await?)
            .map_err(debug)?;
        assert_eq!(balance_of(&harness.pool, ACCOUNT).await?, Some(0));
        harness.cleanup().await?;

        // A backpack with no coin: T = 0 too; the record names the backpack, the event its id.
        let harness = Harness::create(admin, "nocoin", None, Payer::WithBackpack).await?;
        harness.stack(120, "oteryn:item.tibia.i2853", 1, 1).await?;
        harness.balance(ACCOUNT, CHARACTER, 500).await?;
        let second = request(62, 500)?;
        let burned = pay(&harness, 1, &second, V2).await?;
        assert!(burned.lines.is_empty());
        let (_, event, (_, _, item)) = stored_event(&harness.pool, &second).await?;
        assert_eq!(item, None);
        assert_eq!(event.backpack_item_instance_id, id(BACKPACK).to_vec());
        harness.cleanup().await
    })
}

#[test]
fn a_fee_of_the_coins_plus_the_balance_maximum_is_paid_and_one_more_is_refused() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "limit", None, Payer::WithBackpack).await?;
        harness.stack(100, GOLD, 30, 1).await?;
        harness.balance(ACCOUNT, CHARACTER, BALANCE_MAX).await?;
        let before = snapshot(&harness.pool).await?;
        match compose(&harness, 1, &request(61, 30 + BALANCE_MAX + 1)?, V2).await? {
            Composed::Refused(FeeBurnError::InsufficientFunds) => {}
            other => return Err(format!("expected insufficient funds, got {other:?}").into()),
        }
        assert_eq!(snapshot(&harness.pool).await?, before);
        let request = request(62, 30 + BALANCE_MAX)?;
        let burned = pay(&harness, 1, &request, V2).await?;
        assert_eq!(
            burned.bank_debit.map(|debit| debit.debit_gold_units),
            Some(BALANCE_MAX)
        );
        assert_eq!(balance_of(&harness.pool, ACCOUNT).await?, Some(0));
        let (_, event, _) = stored_event(&harness.pool, &request).await?;
        assert_eq!(event.fee_gold_units, 30 + BALANCE_MAX);
        let length: i32 = sqlx::query_scalar(
            "SELECT octet_length(envelope) FROM game_item_audit_outbox WHERE event_id = $1::uuid",
        )
        .bind(hex(&request.event_id))
        .fetch_one(&harness.pool)
        .await?;
        assert!(usize::try_from(length)? <= FEE_RL07_ENVELOPE_BYTES_MAX);
        harness.cleanup().await
    })
}

#[test]
fn the_bank_part_is_an_outcome_recalculated_after_an_abort_and_replayed_after_a_commit()
-> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "replay", None, Payer::WithBackpack).await?;
        harness.stack(100, GOLD, 30, 1).await?;
        harness.balance(ACCOUNT, CHARACTER, 1_000).await?;
        let request = request(61, 100)?;
        let before = snapshot(&harness.pool).await?;
        // A known abort: the first attempt planned 70 from the bank and rolled back.
        match compose_with(&harness, 1, &request, V2, true).await? {
            Composed::Committed(FeeBurnOutcome::Burned(first)) => assert_eq!(
                first.bank_debit.map(|debit| debit.debit_gold_units),
                Some(70)
            ),
            other => return Err(format!("expected a planned bank part, got {other:?}").into()),
        }
        assert_eq!(snapshot(&harness.pool).await?, before);
        // The coins changed before the retry: the same occurrence and binding plan again.
        harness.stack(110, GOLD, 50, 2).await?;
        let retried = pay(&harness, 1, &request, V2).await?;
        assert_eq!(
            retried.bank_debit.map(|debit| debit.debit_gold_units),
            Some(20)
        );
        assert_eq!(balance_of(&harness.pool, ACCOUNT).await?, Some(980));
        // After the commit (an ambiguous one, for the caller) the replay returns that outcome
        // even though the coins changed again.
        harness.stack(120, GOLD, 100, 3).await?;
        let mut tx = harness.runtime.begin().await?;
        let replay = burn_fee_in_transaction_under(&mut tx, &fence(1)?, &request, V2).await;
        tx.rollback().await?;
        match replay {
            Ok(FeeBurnOutcome::AlreadyBurned(first)) => assert_eq!(first, retried),
            other => return Err(format!("expected the retained outcome, got {other:?}").into()),
        }
        harness.cleanup().await
    })
}

/// A fee with a coin-only plan under (2, V2): the bank is not touched.
#[test]
fn a_coin_only_fee_under_v2_writes_no_ledger_entry() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "coins", None, Payer::WithBackpack).await?;
        harness.stack(100, GOLD, 30, 1).await?;
        harness.stack(110, PLATINUM, 5, 2).await?;
        harness.balance(ACCOUNT, CHARACTER, 1_000).await?;
        let request = request(61, 35)?;
        let burned = pay(&harness, 1, &request, V2).await?;
        assert_eq!(burned.bank_debit, None);
        assert_eq!(burned.change_gold_units, 95);
        let (_, event, (revision, _, _)) = stored_event(&harness.pool, &request).await?;
        assert_eq!((revision, event.bank_debit.as_ref()), (2, None));
        assert_eq!(fee_burn_usage(&event).value_lines, 0);
        let entries: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_account_bank_entries WHERE kind = 5")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(entries, 0);
        assert_eq!(balance_of(&harness.pool, ACCOUNT).await?, Some(1_000));
        harness.cleanup().await
    })
}

/// A hand-written bank-only fee as the runtime role: the Character change, the record, the
/// FEE_DEBIT entry and its balance move, and the event without an item.
struct BankFee {
    occurrence: u8,
    fee: u64,
    burned: u64,
    change: u64,
    bank: u64,
    line_count: u32,
    /// The entry: (amount, account, fee reference), or none.
    entry: Option<(u64, u8, String)>,
    /// Extra statements (lines, item changes, entry removals) before the event.
    extra: String,
    event_item: Option<u8>,
}

impl BankFee {
    fn new(occurrence: u8, fee: u64) -> Self {
        Self {
            occurrence,
            fee,
            burned: 0,
            change: 0,
            bank: fee,
            line_count: 0,
            entry: Some((fee, ACCOUNT, uuid(occurrence + 100))),
            extra: String::new(),
            event_item: None,
        }
    }

    /// From a balance of `balance` whose latest entry is the seeded one.
    fn sql(&self, balance: u64) -> String {
        let transaction = uuid(self.occurrence + 100);
        let event = uuid(self.occurrence + 130);
        let entry = match &self.entry {
            Some((amount, account, reference)) => format!(
                "INSERT INTO game_account_bank_entries(entry_id, fee_transaction_id, account_id, \
                   world_id, kind, amount, balance_before, balance_after, previous_entry_id, \
                   acting_character_id) \
                 VALUES ('{entry}'::uuid, {reference}, {account_id}, {world}, 5, {amount}, \
                   {balance}, {after}, '{previous}'::uuid, {character}); \
                 UPDATE game_account_bank_balances SET balance = {after}, \
                   last_entry_id = '{entry}'::uuid WHERE account_id = {account_id};",
                entry = hex(&salted(self.occurrence, 20)),
                account_id = uuid(*account),
                world = uuid(WORLD),
                after = balance - amount,
                previous = hex(&salted(*account, 10)),
                character = uuid(CHARACTER),
            ),
            None => String::new(),
        };
        format!(
            "{change} \
             INSERT INTO game_item_fee_burns(transaction_id, event_id, cause_kind, \
               cause_occurrence_id, charm_key, request_binding, character_id, world_id, \
               channel_id, runtime_scope_ownership_generation, committed_character_revision, \
               backpack_item_instance_id, fee_gold_units, burned_gold_units, change_gold_units, \
               line_count, occurred_at, envelope_sha256, committed_at, bank_debit_gold_units) \
             VALUES ({transaction}, {event}, 1, {occurrence_id}, 'oteryn:charm.c{occurrence}', \
               '\\x{binding}'::bytea, {character}, {world}, {channel}, 1, 2, {backpack}, {fee}, \
               {burned}, {change_units}, {lines}, {OCCURRED_AT}, sha256('\\x0102'::bytea), 1, \
               {bank}); \
             {entry} {extra} \
             INSERT INTO game_item_audit_outbox(event_id, transaction_id, transaction_ordinal, \
               transaction_count, event_type_id, schema_revision, retention_profile_id, \
               item_instance_id, occurred_at, expires_at, envelope, envelope_sha256, \
               publication_state) \
             VALUES ({event}, {transaction}, 1, 1, 2, 2, \
               'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2', {item}, {OCCURRED_AT}, \
               {OCCURRED_AT} + 7776000000, '\\x0102'::bytea, sha256('\\x0102'::bytea), 1);",
            change = character_change(self.occurrence, 1),
            occurrence = self.occurrence,
            occurrence_id = uuid(self.occurrence),
            binding = hex(&[self.occurrence; 32]),
            character = uuid(CHARACTER),
            world = uuid(WORLD),
            channel = uuid(CHANNEL),
            backpack = uuid(BACKPACK),
            fee = self.fee,
            burned = self.burned,
            change_units = self.change,
            lines = self.line_count,
            bank = self.bank,
            extra = self.extra,
            item = self.event_item.map_or_else(|| "NULL".into(), uuid),
        )
    }
}

async fn outcome(harness: &Harness, script: &str) -> TestResult<Result<(), String>> {
    let mut tx = harness.runtime.begin().await?;
    let result = match sqlx::raw_sql(sqlx::AssertSqlSafe(script.to_owned()))
        .execute(&mut *tx)
        .await
    {
        Ok(_) => tx.commit().await,
        Err(error) => Err(error),
    };
    Ok(result.map_err(|error| sqlstate(&error)))
}

async fn expect_rejected(harness: &Harness, case: &str, script: &str) -> TestResult {
    let before = snapshot(&harness.pool).await?;
    match outcome(harness, script).await? {
        Err(code) if code == "23514" || code == "23503" => {}
        other => return Err(format!("{case}: expected a refused commit, got {other:?}").into()),
    }
    if snapshot(&harness.pool).await? != before {
        return Err(format!("{case}: a refused transaction changed durable state").into());
    }
    Ok(())
}

/// One whole or partial gold line of a hand-written fee, with its item change and entry removal.
fn sql_line(
    occurrence: u8,
    ordinal: u16,
    item: u8,
    placement: u64,
    before: u32,
    after: u32,
) -> String {
    let transaction = uuid(occurrence + 100);
    format!(
        "INSERT INTO game_item_fee_burn_lines(transaction_id, line_ordinal, item_instance_id, \
           placement_ordinal, coin_worth, quantity_before, quantity_after) \
         VALUES ({transaction}, {ordinal}, {item_id}, {placement}, 1, {before}, {after}); \
         UPDATE game_item_instances SET quantity = {after}, lifecycle = {lifecycle}, \
           last_transaction_id = {transaction} WHERE item_instance_id = {item_id}; {delete}",
        item_id = uuid(item),
        lifecycle = if after == 0 { 2 } else { 1 },
        delete = if after == 0 {
            format!(
                "DELETE FROM game_item_container_entries WHERE item_instance_id = {};",
                uuid(item)
            )
        } else {
            String::new()
        },
    )
}

/// The exact statements a writer issues, as the runtime role, with one invariant broken per case.
#[test]
fn the_database_binds_every_bank_part_to_its_coins_first_plan_and_ledger_entry() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin.clone(), "guards", None, Payer::WithBackpack).await?;
        harness.balance(ACCOUNT, CHARACTER, 1_000).await?;
        harness
            .balance(OTHER_ACCOUNT, OTHER_CHARACTER, 1_000)
            .await?;

        let mut cases: Vec<(&str, String)> = Vec::new();
        let mut fee = BankFee::new(61, 500);
        fee.entry = None;
        cases.push(("a bank part and no FEE_DEBIT", fee.sql(1_000)));
        let mut fee = BankFee::new(61, 500);
        fee.entry = Some((499, ACCOUNT, uuid(161)));
        cases.push(("a FEE_DEBIT of another amount", fee.sql(1_000)));
        let mut fee = BankFee::new(61, 500);
        fee.entry = Some((500, OTHER_ACCOUNT, uuid(161)));
        cases.push(("a FEE_DEBIT on another Account", fee.sql(1_000)));
        let mut fee = BankFee::new(61, 500);
        fee.change = 1;
        fee.burned = 1;
        cases.push(("change with a bank part", fee.sql(1_000)));
        let mut fee = BankFee::new(61, 500);
        fee.fee = 501;
        cases.push(("conservation broken", fee.sql(1_000)));
        let mut fee = BankFee::new(61, 500);
        fee.bank = 0;
        fee.entry = None;
        cases.push(("no line and no bank part", fee.sql(1_000)));
        for (case, script) in &cases {
            expect_rejected(&harness, case, script).await?;
        }
        // An item-less event that is not a bank-only fee's.
        expect_rejected(
            &harness,
            "an item-less event without its fee record",
            &format!(
                "INSERT INTO game_item_audit_outbox(event_id, transaction_id, \
                   transaction_ordinal, transaction_count, event_type_id, schema_revision, \
                   retention_profile_id, item_instance_id, occurred_at, expires_at, envelope, \
                   envelope_sha256, publication_state) \
                 VALUES ({}, {}, 1, 1, 2, 1, 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1', NULL, \
                   {OCCURRED_AT}, {OCCURRED_AT} + 7776000000, '\\x0102'::bytea, \
                   sha256('\\x0102'::bytea), 1);",
                uuid(190),
                uuid(191)
            ),
        )
        .await?;

        // The positive control: the same statements, unbroken, commit.
        if let Err(code) = outcome(&harness, &BankFee::new(61, 500).sql(1_000)).await? {
            return Err(format!("the valid bank-only fee was refused: {code}").into());
        }
        assert_eq!(balance_of(&harness.pool, ACCOUNT).await?, Some(500));

        // A FEE_DEBIT whose fee committed earlier, in another transaction: a coin-only fee.
        harness.stack(100, GOLD, 30, 1).await?;
        pay(&harness, 2, &request(63, 30)?, V1).await?;
        let entry_only = format!(
            "INSERT INTO game_account_bank_entries(entry_id, fee_transaction_id, account_id, \
               world_id, kind, amount, balance_before, balance_after, previous_entry_id, \
               acting_character_id) \
             VALUES ('{entry}'::uuid, {fee}, {account}, {world}, 5, 500, 500, 0, \
               '{previous}'::uuid, {character}); \
             UPDATE game_account_bank_balances SET balance = 0, last_entry_id = '{entry}'::uuid \
              WHERE account_id = {account};",
            entry = hex(&salted(62, 20)),
            fee = uuid(163),
            account = uuid(ACCOUNT),
            world = uuid(WORLD),
            previous = hex(&salted(61, 20)),
            character = uuid(CHARACTER),
        );
        expect_rejected(&harness, "a FEE_DEBIT with no fee of its own", &entry_only).await?;
        let dangling = entry_only.replace(&uuid(163), &uuid(199));
        expect_rejected(&harness, "a FEE_DEBIT naming no fee record", &dangling).await?;
        // The runtime role cannot delete a ledger entry.
        let deleted = outcome(
            &harness,
            "DELETE FROM game_account_bank_entries WHERE kind = 5",
        )
        .await?;
        assert!(deleted.is_err(), "{deleted:?}");
        harness.cleanup().await?;

        // Coins first: a bank part with an untouched or a partly burned coin stack is refused.
        let harness = Harness::create(admin, "first", None, Payer::WithBackpack).await?;
        harness.balance(ACCOUNT, CHARACTER, 1_000).await?;
        harness.stack(100, GOLD, 30, 1).await?;
        expect_rejected(
            &harness,
            "an untouched eligible input",
            &BankFee::new(61, 500).sql(1_000),
        )
        .await?;
        let mut partial = BankFee::new(61, 500);
        partial.line_count = 1;
        partial.burned = 20;
        partial.bank = 480;
        partial.entry = Some((480, ACCOUNT, uuid(161)));
        partial.extra = sql_line(61, 1, 100, 1, 30, 10);
        partial.event_item = Some(100);
        expect_rejected(
            &harness,
            "a partial line with a bank part",
            &partial.sql(1_000),
        )
        .await?;
        let mut all = BankFee::new(61, 500);
        all.line_count = 1;
        all.burned = 30;
        all.bank = 470;
        all.entry = Some((470, ACCOUNT, uuid(161)));
        all.extra = sql_line(61, 1, 100, 1, 30, 0);
        all.event_item = Some(100);
        if let Err(code) = outcome(&harness, &all.sql(1_000)).await? {
            return Err(format!("the valid coins-then-bank fee was refused: {code}").into());
        }
        assert_eq!(balance_of(&harness.pool, ACCOUNT).await?, Some(530));
        harness.cleanup().await
    })
}

/// A synthetic outbox row as the migration owner with the deferred proofs off; CHECKs still run.
fn outbox_row(seed: u8, revision: i64, profile: &str, envelope: &[u8]) -> String {
    format!(
        "INSERT INTO game_item_audit_outbox(event_id, transaction_id, transaction_ordinal, \
           transaction_count, event_type_id, schema_revision, retention_profile_id, \
           item_instance_id, occurred_at, expires_at, envelope, envelope_sha256, \
           publication_state) \
         VALUES ('{}'::uuid, '{}'::uuid, 1, 1, 2, {revision}, '{profile}', {}, {OCCURRED_AT}, \
           {OCCURRED_AT} + 7776000000, '\\x{bytes}'::bytea, sha256('\\x{bytes}'::bytea), 1);",
        hex(&salted(seed, 30)),
        hex(&salted(seed, 31)),
        uuid(BACKPACK),
        bytes = hex(envelope),
    )
}

#[test]
fn rows_stored_before_0072_verify_as_v1_and_only_the_two_tuples_are_admitted() -> TestResult {
    run(async |admin| {
        // A database at 0071 (`main` before GOLD-FEE-2) holding a (1, V1) row of every shape.
        let harness = Harness::create(admin, "tuple", Some(71), Payer::WithBackpack).await?;
        let mut script = String::new();
        for (seed, (_, hex_envelope)) in (1_u8..).zip(TYPE2_GOLDEN_V1) {
            script.push_str(&outbox_row(
                seed,
                1,
                "DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1",
                &unhex(hex_envelope),
            ));
        }
        harness.seed(&script).await?;
        // GOLD-FEE-2's migration validates its tuple CHECK against every stored row.
        let mut connection = sqlx::PgConnection::connect(&harness.url).await?;
        sqlx::migrate!("./migrations").run(&mut connection).await?;
        connection.close().await?;
        for (seed, (shape, _)) in (1_u8..).zip(TYPE2_GOLDEN_V1) {
            let (envelope, revision, profile): (Vec<u8>, i64, String) = sqlx::query_as(
                "SELECT envelope, schema_revision, retention_profile_id \
                   FROM game_item_audit_outbox WHERE event_id = $1::uuid",
            )
            .bind(hex(&salted(seed, 30)))
            .fetch_one(&harness.pool)
            .await?;
            let decoded = verify_type2_shape(shape, &envelope).map_err(debug)?;
            assert_eq!(
                Type2EventTuple::of(decoded.event_schema_revision, &decoded.retention_profile_id),
                Some(V1),
                "{shape}"
            );
            assert_eq!(
                (revision, profile.as_str()),
                (1, "DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1")
            );
        }
        // A new (2, V2) row passes the CHECK and verifies as V2.
        let (shape, golden) = TYPE2_GOLDEN_V1[0];
        let mut v2 = EventEnvelopeV1::decode(unhex(golden).as_slice())?;
        v2.event_schema_revision = 2;
        v2.retention_profile_id = "DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2".into();
        let v2 = v2.encode_to_vec();
        harness
            .seed(&outbox_row(
                100,
                2,
                "DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2",
                &v2,
            ))
            .await?;
        let stored: Vec<u8> = sqlx::query_scalar(
            "SELECT envelope FROM game_item_audit_outbox WHERE event_id = $1::uuid",
        )
        .bind(hex(&salted(100, 30)))
        .fetch_one(&harness.pool)
        .await?;
        let decoded = verify_type2_shape(shape, &stored).map_err(debug)?;
        assert_eq!(decoded.event_schema_revision, 2);
        // (1, V2), (2, V1), revision 3 and a profile that is neither are refused by the CHECK.
        for (seed, revision, profile) in [
            (101, 1, "DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2"),
            (102, 2, "DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1"),
            (103, 3, "DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2"),
            (104, 2, "DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V3"),
            (105, 1, "ECONOMY_LEDGER_RETENTION_V1"),
        ] {
            let refused = harness
                .seed(&outbox_row(seed, revision, profile, &v2))
                .await
                .err()
                .map(|error| format!("{error}"));
            assert!(
                refused
                    .as_deref()
                    .is_some_and(|text| text.contains("game_item_audit_outbox_event_tuple")),
                "({revision}, {profile}) was admitted: {refused:?}"
            );
        }
        harness.cleanup().await
    })
}
