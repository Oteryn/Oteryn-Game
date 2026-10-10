#![allow(clippy::expect_used, clippy::unwrap_used)]
// GOLD-FEE-ACT-1 cases (migration 0079, decision GOLD-FEE-ACT-PACKET-1 §2.1): the activation
// table, the shared fence of every type-2 transaction, the outbox backstop with its exact
// grandfather exception, and the reservation marker. The activation row is inserted by the
// migration owner, as GOLD-FEE-ACT-2 will. Ordinary workspace runs report
// PRE-ROUTING/NONCANONICAL when the routed database is absent.

use crate::character_recovery_fence::CharacterRecoveryStore;
use crate::durability::DurabilityRoot;
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCandidate, ItemMintCause, ItemMintOutcome, ItemMintRequest,
    TypedDefinitionRef,
};
use crate::durability::item_mint_audit::{
    self as audit, TYPE2_AUDIT_ACTIVATION_FENCE, Type2EventTuple, Type2Transaction,
};
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, BootstrapSecret, ControlActor,
    LaunchBinding, NodeIncarnationProof, OperationKey, RuntimeScopeAssignmentWriter,
};
use crate::foundation::{ChannelId, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId};
use sqlx::{Connection, Executor, PgPool, Row};
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const WORLD: u8 = 42;
const CHANNEL: u8 = 43;
/// The last migration before 0079, for the backfill case.
const BEFORE_ACTIVATION_SCHEMA: i64 = 78;
const ACTIVATE: &str = "INSERT INTO game_type2_audit_activation VALUES (1, now())";

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

fn uuid(seed: u8) -> String {
    let hex: String = id(seed).iter().map(|byte| format!("{byte:02x}")).collect();
    format!("'{hex}'::uuid")
}

fn debug<E: std::fmt::Debug>(error: E) -> String {
    format!("{error:?}")
}

fn sqlstate(error: &sqlx::Error) -> String {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .map_or_else(|| format!("{error}"), |code| code.into_owned())
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

struct Database {
    admin_url: String,
    name: String,
    url: String,
}

impl Database {
    /// A fresh database migrated through `upto` (every migration when `None`).
    async fn create(admin_url: String, tag: &str, upto: Option<i64>) -> TestResult<Self> {
        if !admin_url.starts_with("postgresql://oteryn_test_admin:")
            || !admin_url.ends_with("@127.0.0.1:5432/postgres")
        {
            return Err("unsafe PostgreSQL test admin URL".into());
        }
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = format!("t2act_{tag}_{suffix}");
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
        let database = Self {
            admin_url,
            name,
            url,
        };
        database.migrate(upto).await?;
        Ok(database)
    }

    async fn migrate(&self, upto: Option<i64>) -> TestResult {
        let mut connection = sqlx::PgConnection::connect(&self.url).await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&mut connection)
            .await?;
        assert!(
            version.starts_with("17"),
            "PostgreSQL 17 required: {version}"
        );
        let migrator = sqlx::migrate!("./migrations");
        match upto {
            Some(version) => migrator.run_to(version, &mut connection).await?,
            None => migrator.run(&mut connection).await?,
        }
        connection.close().await?;
        Ok(())
    }

    /// A login in the runtime group (0006).
    async fn runtime_login(&self, owner: &PgPool, tag: &str) -> TestResult<PgPool> {
        let role = format!("{}_{tag}", self.name);
        let password = format!("{role}-secret");
        for statement in [
            format!("CREATE ROLE {role} LOGIN PASSWORD '{password}' IN ROLE oteryn_game_runtime"),
            format!("GRANT CONNECT ON DATABASE {} TO {role}", self.name),
        ] {
            sqlx::query(sqlx::AssertSqlSafe(statement))
                .execute(owner)
                .await?;
        }
        let (_, address) = self.url.split_once('@').ok_or("no authority separator")?;
        Ok(PgPool::connect(&format!("postgresql://{role}:{password}@{address}")).await?)
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

/// The DUR-03 Ground MINT writer on a fully migrated database: the reference type-2 writer whose
/// reservation persists its exact envelope.
struct Harness {
    database: Database,
    root: DurabilityRoot,
    /// The migration owner, for fixtures, activation and read-back.
    pool: PgPool,
    /// A runtime login, for raw type-2 transactions.
    runtime: PgPool,
    recovery: CharacterRecoveryStore,
    retained: std::path::PathBuf,
    node: NodeIncarnationProof,
    writer: RuntimeScopeAssignmentWriter,
}

impl Harness {
    async fn create(admin: String, tag: &str) -> TestResult<Self> {
        let database = Database::create(admin, tag, None).await?;
        let root = DurabilityRoot::connect_test_runtime(&database.url)?;
        assert!(root.maintain_ready_once().await?);
        let pool = PgPool::connect(&database.url).await?;
        let runtime = database.runtime_login(&pool, "rt").await?;
        let retained = fence_parent(tag)?;
        let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
            .map_err(debug)?;
        {
            let fresh = recovery.authorize_fresh_store(id(10), 100).map_err(debug)?;
            root.admit_fresh_character_recovery(&fresh)
                .await
                .map_err(debug)?;
        }
        let node = register(&root).await?;
        sqlx::query(
            "INSERT INTO game_control_scope_grants \
             (control_role, world_id, channel_id, operation) \
             SELECT session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, operation \
               FROM unnest(ARRAY[1, 2, 3]::SMALLINT[]) AS operation",
        )
        .bind(id(WORLD).as_slice())
        .bind(id(CHANNEL).as_slice())
        .execute(&pool)
        .await?;
        let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "type2-activation-writer")
            .await
            .map_err(debug)?;
        let outcome = writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([7_u8; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Assign {
                    scope: RuntimeScopeRefV1::channel(
                        WorldId::decode(&id(WORLD)).map_err(debug)?,
                        ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
                    ),
                    target: node.fact(),
                },
            })
            .await
            .map_err(debug)?;
        let AssignmentOutcome::Committed(_) = outcome else {
            return Err(format!("unexpected assignment outcome: {outcome:?}").into());
        };
        Ok(Self {
            database,
            root,
            pool,
            runtime,
            recovery,
            retained,
            node,
            writer,
        })
    }

    async fn freeze(&self, actor: u32) -> TestResult<ItemMintCandidate> {
        let seal = self.recovery.seal_current().map_err(debug)?;
        let authority = self
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        Ok(self
            .root
            .freeze_item_mint(&authority, &self.node, request(actor)?)
            .await
            .map_err(debug)?)
    }

    async fn commit(&self, candidate: &mut ItemMintCandidate) -> TestResult {
        let seal = self.recovery.seal_current().map_err(debug)?;
        let authority = self
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        match self
            .root
            .commit_item_mint(
                &candidate.fresh_death_lane_permit().await,
                &authority,
                &self.node,
                candidate,
            )
            .await
            .map_err(debug)?
        {
            ItemMintOutcome::Committed(_) => Ok(()),
            other => Err(format!("expected a fresh commit, got {other:?}").into()),
        }
    }

    async fn activate(&self) -> TestResult {
        sqlx::query(ACTIVATE).execute(&self.pool).await?;
        Ok(())
    }

    /// The stored tuple of `candidate`'s event and its reservation's marker.
    async fn stored(&self, candidate: &ItemMintCandidate) -> TestResult<Stored> {
        let outbox = sqlx::query(
            "SELECT schema_revision, retention_profile_id, envelope FROM game_item_audit_outbox \
              WHERE event_id = encode($1,'hex')::uuid",
        )
        .bind(candidate.event_id().as_slice())
        .fetch_one(&self.pool)
        .await?;
        let reservation = sqlx::query(
            "SELECT type2_schema_revision, type2_pre_activation, envelope \
               FROM game_item_mint_reservations WHERE event_id = encode($1,'hex')::uuid",
        )
        .bind(candidate.event_id().as_slice())
        .fetch_one(&self.pool)
        .await?;
        let envelope: Vec<u8> = outbox.try_get("envelope")?;
        assert_eq!(
            envelope,
            reservation.try_get::<Vec<u8>, _>("envelope")?,
            "the event carries the reservation's exact bytes"
        );
        assert_eq!(envelope.as_slice(), candidate.envelope());
        let (decoded, _) = audit::decode_envelope(&envelope).map_err(debug)?;
        let schema_revision = u32::try_from(outbox.try_get::<i64, _>("schema_revision")?)?;
        let profile: String = outbox.try_get("retention_profile_id")?;
        let tuple = Type2EventTuple::of(schema_revision, &profile).ok_or("unknown tuple")?;
        assert_eq!(
            Type2EventTuple::of(decoded.event_schema_revision, &decoded.retention_profile_id),
            Some(tuple),
            "the row tuple is the envelope's"
        );
        Ok(Stored {
            tuple,
            reservation_revision: reservation.try_get("type2_schema_revision")?,
            pre_activation: reservation.try_get("type2_pre_activation")?,
        })
    }

    /// Re-inserts a copy of `candidate`'s stored event as the migration owner after `change`
    /// (SQL `SET` items on the copy, or none) and returns the SQLSTATE. An admitted copy then
    /// fails its primary key (`23505`); a refused one fails the type-2 guard (`OTA01`).
    async fn reinsert(&self, candidate: &ItemMintCandidate, change: &str) -> TestResult<String> {
        let mut tx = self.pool.begin().await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "CREATE TEMP TABLE copied ON COMMIT DROP AS SELECT * FROM game_item_audit_outbox \
              WHERE event_id = '{}'::uuid",
            hex(candidate.event_id())
        )))
        .execute(&mut *tx)
        .await?;
        if !change.is_empty() {
            sqlx::raw_sql(sqlx::AssertSqlSafe(format!("UPDATE copied SET {change}")))
                .execute(&mut *tx)
                .await?;
        }
        let error = sqlx::query("INSERT INTO game_item_audit_outbox SELECT * FROM copied")
            .execute(&mut *tx)
            .await
            .expect_err("a copied event is never admitted");
        tx.rollback().await?;
        Ok(sqlstate(&error))
    }

    async fn cleanup(self) -> TestResult {
        drop(self.writer);
        self.runtime.close().await;
        self.pool.close().await;
        self.database.cleanup().await?;
        std::fs::remove_dir_all(self.retained)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Stored {
    tuple: Type2EventTuple,
    reservation_revision: Option<i16>,
    pre_activation: bool,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn fence_parent(tag: &str) -> TestResult<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let parent = std::env::temp_dir().join(format!(
        "oteryn-type2-activation-parent-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&parent)?;
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700))?;
    let retained = parent.join(tag);
    let _ = std::fs::remove_dir_all(&retained);
    std::fs::create_dir(&retained)?;
    std::fs::set_permissions(&retained, std::fs::Permissions::from_mode(0o700))?;
    Ok(retained)
}

async fn register(root: &DurabilityRoot) -> TestResult<NodeIncarnationProof> {
    let secret = BootstrapSecret::from_bytes([1; 32]);
    let launch = LaunchBinding::new("type2-activation-launch").map_err(debug)?;
    let node = crate::foundation::NodeId::decode(&id(1)).map_err(debug)?;
    root.issue_node_bootstrap_authorization(&secret, &launch, None)
        .await
        .map_err(debug)?;
    root.register_node_incarnation(&secret, &launch, node)
        .await
        .map_err(|error| debug(error).into())
}

fn definition(family: &str, key: &str, revision: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: family.into(),
        production_key: key.into(),
        revision_ref: revision.into(),
    }
}

/// One Ground MINT per `actor`, each its own cause.
fn request(actor: u32) -> TestResult<ItemMintRequest> {
    Ok(ItemMintRequest {
        cause: ItemMintCause::for_test(
            WorldId::decode(&id(WORLD)).map_err(debug)?,
            ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            ScopeOwnershipGeneration::new(1).map_err(debug)?,
            actor,
            1,
            definition("LootTable", "fixture:loot.alpha", "loot-r1"),
            "fixture:purpose.drop".into(),
            1,
        ),
        item: definition("ItemType", "fixture:alpha", "rev-a/1"),
        quantity: 1,
        ground: GroundPlacement {
            spatial_position: vec![1, 2, 3],
            corpse_ref: id(3).to_vec(),
            map_revision: "map-1".into(),
            content_revision: "content-1".into(),
            native_room_placement_context: id(6).to_vec(),
        },
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    })
}

/// A raw type-2 transaction as the runtime login: the tuple it selected, then rolled back.
async fn open_tuple(runtime: PgPool) -> Result<Type2EventTuple, sqlx::Error> {
    let tx = Type2Transaction::open(runtime.begin().await?).await?;
    let tuple = tx.tuple().tuple();
    tx.into_inner().rollback().await?;
    Ok(tuple)
}

/// A raw mint reservation as the migration owner with every trigger on, `seed` choosing its
/// identities, `revision` its writer-set revision and `forged` the marker it supplies.
fn reservation(seed: u8, revision: &str, forged: bool) -> String {
    format!(
        "INSERT INTO game_item_mint_reservations \
           (death_world_id, death_channel_id, death_scope_ownership_generation, \
            death_actor_local_id, death_actor_local_generation, loot_table_family, \
            loot_table_production_key, loot_table_revision_ref, loot_purpose_key, \
            draw_ordinal, intent_binding, transaction_id, event_id, item_instance_id, \
            occurred_at, envelope, fence_scope_ownership_generation, fence_holder_node_id, \
            fence_holder_registration_revision, work_units_used, reserved_at, \
            type2_schema_revision, type2_pre_activation) \
         VALUES ({world}, {channel}, 1, {seed}, 1, 'LootTable', 'fixture:loot.forged', 'loot-r1', \
            'fixture:purpose.drop', 1, decode(repeat('cd',33),'hex'), {tx}, {ev}, {item}, 1000, \
            decode(repeat('ab',16),'hex'), 1, {node}, 0, 0, 900, {revision}, {forged})",
        world = uuid(WORLD),
        channel = uuid(CHANNEL),
        tx = uuid(seed),
        ev = uuid(seed + 1),
        item = uuid(seed + 2),
        node = uuid(1),
    )
}

async fn marker_of(pool: &PgPool, seed: u8) -> TestResult<(Option<i16>, bool)> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT type2_schema_revision, type2_pre_activation FROM game_item_mint_reservations \
          WHERE event_id = {}",
        uuid(seed + 1)
    )))
    .fetch_one(pool)
    .await?;
    Ok((
        row.try_get("type2_schema_revision")?,
        row.try_get("type2_pre_activation")?,
    ))
}

async fn refusal(pool: &PgPool, statement: &str) -> TestResult<(String, String)> {
    let error = sqlx::raw_sql(sqlx::AssertSqlSafe(statement.to_owned()))
        .execute(pool)
        .await
        .expect_err("the statement is refused");
    let message = error
        .as_database_error()
        .map(|error| error.message().to_owned())
        .unwrap_or_default();
    Ok((sqlstate(&error), message))
}

/// §2.1: an empty activation table selects `(1, V1)` and the row selects `(2, V2)`, for the event
/// and the reservation alike; a candidate frozen before activation commits with its persisted V1
/// bytes, and the backstop admits exactly that event and nothing else.
#[test]
fn the_activation_row_selects_the_tuple_and_admits_only_exact_frozen_v1() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "tuple").await?;

        let mut before = harness.freeze(1).await?;
        harness.commit(&mut before).await?;
        assert_eq!(
            harness.stored(&before).await?,
            Stored {
                tuple: Type2EventTuple::V1,
                reservation_revision: Some(1),
                pre_activation: true,
            }
        );
        let mut frozen = harness.freeze(2).await?;
        // Before activation a V2 copy is refused and the V1 copy is admitted (then a duplicate).
        assert_eq!(
            harness
                .reinsert(
                    &before,
                    "schema_revision = 2, \
                     retention_profile_id = 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2'"
                )
                .await?,
            "OTA01"
        );
        assert_eq!(harness.reinsert(&before, "").await?, "23505");
        assert_eq!(
            open_tuple(harness.runtime.clone()).await?,
            Type2EventTuple::V1
        );

        harness.activate().await?;
        assert_eq!(
            open_tuple(harness.runtime.clone()).await?,
            Type2EventTuple::V2
        );

        // The candidate frozen before activation commits with its exact persisted V1 bytes.
        harness.commit(&mut frozen).await?;
        assert_eq!(
            harness.stored(&frozen).await?,
            Stored {
                tuple: Type2EventTuple::V1,
                reservation_revision: Some(1),
                pre_activation: true,
            }
        );
        let mut after = harness.freeze(3).await?;
        harness.commit(&mut after).await?;
        assert_eq!(
            harness.stored(&after).await?,
            Stored {
                tuple: Type2EventTuple::V2,
                reservation_revision: Some(2),
                pre_activation: false,
            }
        );

        // The grandfather is exact: the frozen event_id with its frozen digest only.
        assert_eq!(harness.reinsert(&frozen, "").await?, "23505");
        assert_eq!(
            harness
                .reinsert(
                    &frozen,
                    "envelope = envelope || '\\x00'::bytea, \
                     envelope_sha256 = sha256(envelope || '\\x00'::bytea)"
                )
                .await?,
            "OTA01"
        );
        assert_eq!(
            harness
                .reinsert(
                    &frozen,
                    "event_id = '019a0000-0000-7000-8000-000000000001'::uuid"
                )
                .await?,
            "OTA01"
        );
        // A post-activation reservation never grandfathers V1, and V2 stays admitted.
        assert_eq!(
            harness
                .reinsert(
                    &after,
                    "schema_revision = 1, \
                     retention_profile_id = 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1'"
                )
                .await?,
            "OTA01"
        );
        assert_eq!(harness.reinsert(&after, "").await?, "23505");
        // An older binary's V1 event for a cause it never froze before activation is refused
        // by the backstop although it takes no fence itself.
        assert_eq!(
            harness
                .reinsert(
                    &before,
                    "event_id = '019a0000-0000-7000-8000-000000000002'::uuid"
                )
                .await?,
            "OTA01"
        );
        harness.cleanup().await
    })
}

/// §2.1: a type-2 transaction in flight holds activation off; a writer that opens while
/// activation holds the fence waits and then selects V2; shared holders on several nodes never
/// wait for each other; a writer whose deadline runs out while waiting writes nothing.
#[test]
fn the_fence_orders_writers_and_activation() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "fence").await?;
        let second_node = harness.database.runtime_login(&harness.pool, "rt2").await?;

        // Two nodes in flight at once: shared holders do not wait for each other.
        let first = Type2Transaction::open(harness.runtime.begin().await?).await?;
        let second = tokio::time::timeout(
            Duration::from_secs(1),
            Type2Transaction::open(second_node.begin().await?),
        )
        .await??;
        assert_eq!(first.tuple().tuple(), Type2EventTuple::V1);
        assert_eq!(second.tuple().tuple(), Type2EventTuple::V1);
        second.into_inner().rollback().await?;

        // Activation cannot take the fence while a writer holds it.
        let mut activation = harness.pool.begin().await?;
        sqlx::query("SET LOCAL lock_timeout = '300ms'")
            .execute(&mut *activation)
            .await?;
        let blocked = sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(TYPE2_AUDIT_ACTIVATION_FENCE)
            .execute(&mut *activation)
            .await
            .expect_err("activation waits for the in-flight writer");
        assert_eq!(sqlstate(&blocked), "55P03");
        activation.rollback().await?;
        first.into_inner().rollback().await?;

        // Activation holds the fence exclusive with its row not yet committed.
        let mut activation = harness.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(TYPE2_AUDIT_ACTIVATION_FENCE)
            .execute(&mut *activation)
            .await?;
        sqlx::query(ACTIVATE).execute(&mut *activation).await?;
        let waiting = tokio::spawn(open_tuple(harness.runtime.clone()));

        // A writer whose pass deadline runs out while waiting writes nothing.
        let started = std::time::Instant::now();
        let timed_out = harness.freeze(9).await.expect_err("the fence is held");
        // The pass's own `lock_timeout` (its remaining deadline) or the deadline itself ends
        // the wait; either way the pass is refused.
        assert!(
            ["Deadline", "Unavailable"]
                .iter()
                .any(|refusal| timed_out.to_string().contains(refusal)),
            "unexpected refusal: {timed_out}"
        );
        assert!(started.elapsed() < Duration::from_secs(5));
        let reservations: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_item_mint_reservations")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(reservations, 0);

        assert!(!waiting.is_finished(), "the writer waits for activation");
        activation.commit().await?;
        assert_eq!(waiting.await??, Type2EventTuple::V2);

        second_node.close().await;
        harness.cleanup().await
    })
}

/// §2.1: the reservation marker is the database's, written under the fence: a supplied marker is
/// overwritten, a revision that does not match the activation (an older binary's NULL included)
/// is refused, and neither column can change afterwards.
#[test]
fn the_reservation_marker_is_set_under_the_fence_and_immutable() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "marker").await?;
        let pool = &harness.pool;

        sqlx::raw_sql(sqlx::AssertSqlSafe(reservation(110, "1", false)))
            .execute(pool)
            .await?;
        assert_eq!(marker_of(pool, 110).await?, (Some(1), true), "forged false");
        sqlx::raw_sql(sqlx::AssertSqlSafe(reservation(120, "NULL", false)))
            .execute(pool)
            .await?;
        assert_eq!(marker_of(pool, 120).await?, (None, true), "older binary");
        let (code, _) = refusal(pool, &reservation(130, "2", true)).await?;
        assert_eq!(code, "OTA01", "V2 before activation");

        harness.activate().await?;
        for (seed, revision) in [(140, "1"), (150, "NULL")] {
            let (code, message) = refusal(pool, &reservation(seed, revision, true)).await?;
            assert_eq!(code, "OTA01", "revision {revision} after activation");
            assert!(message.contains("reservation revision"), "{message}");
        }
        sqlx::raw_sql(sqlx::AssertSqlSafe(reservation(160, "2", true)))
            .execute(pool)
            .await?;
        assert_eq!(marker_of(pool, 160).await?, (Some(2), false), "forged true");

        for (seed, change) in [
            (110, "type2_pre_activation = false"),
            (160, "type2_pre_activation = true"),
            (110, "type2_schema_revision = 2"),
            (120, "type2_schema_revision = 1"),
        ] {
            let (code, message) = refusal(
                pool,
                &format!(
                    "UPDATE game_item_mint_reservations SET {change} WHERE event_id = {}",
                    uuid(seed + 1)
                ),
            )
            .await?;
            // DUR-03's own row guard runs first here; the decay-retire case below reaches the
            // marker guard itself.
            assert_eq!(code, "23514", "{change}");
            assert!(message.contains("immutable"), "{message}");
        }
        // The runtime role cannot touch the marker at all (only `work_units_used`).
        let (code, _) = refusal(
            &harness.runtime,
            &format!(
                "UPDATE game_item_mint_reservations SET type2_pre_activation = true \
                  WHERE event_id = {}",
                uuid(161)
            ),
        )
        .await?;
        assert_eq!(code, "42501");

        // The decay-retire reservation carries the same triggers.
        let triggers: Vec<String> = sqlx::query_scalar(
            "SELECT tgname::text FROM pg_trigger \
              WHERE tgrelid = 'game_item_decay_retire_reservations'::regclass \
                AND tgname LIKE '%type2%' ORDER BY tgname",
        )
        .fetch_all(pool)
        .await?;
        assert_eq!(
            triggers,
            [
                "game_item_decay_retire_reservations_type2_columns",
                "game_item_decay_retire_reservations_type2_marker",
            ]
        );
        harness.cleanup().await
    })
}

/// §2.1: the activation table holds at most the one row, which the runtime cannot write and
/// nobody can change, delete or truncate.
#[test]
fn the_activation_row_is_insert_only_and_owner_only() -> TestResult {
    run(async |admin| {
        let database = Database::create(admin, "row", None).await?;
        let pool = PgPool::connect(&database.url).await?;
        let runtime = database.runtime_login(&pool, "rt").await?;

        let (code, _) = refusal(&runtime, ACTIVATE).await?;
        assert_eq!(code, "42501", "the runtime cannot activate");
        let (code, _) = refusal(
            &pool,
            "INSERT INTO game_type2_audit_activation VALUES (2, now())",
        )
        .await?;
        assert_eq!(code, "23514");
        sqlx::query(ACTIVATE).execute(&pool).await?;
        let (code, _) = refusal(&pool, ACTIVATE).await?;
        assert_eq!(code, "23505");
        for statement in [
            "UPDATE game_type2_audit_activation SET activated_at = now()",
            "DELETE FROM game_type2_audit_activation",
            "TRUNCATE game_type2_audit_activation",
        ] {
            let (code, _) = refusal(&pool, statement).await?;
            assert_eq!(code, "23514", "{statement}");
        }
        let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM game_type2_audit_activation")
            .fetch_one(&runtime)
            .await?;
        assert_eq!(rows, 1, "the runtime reads the activation");

        runtime.close().await;
        pool.close().await;
        database.cleanup().await
    })
}

/// §2.1: every reservation that exists when 0079 applies predates activation, so it is backfilled
/// with the marker true and no writer revision.
#[test]
fn existing_reservations_are_backfilled_as_pre_activation() -> TestResult {
    run(async |admin| {
        let database = Database::create(admin, "backfill", Some(BEFORE_ACTIVATION_SCHEMA)).await?;
        let pool = PgPool::connect(&database.url).await?;
        let mint = reservation(170, "", false);
        let mint = mint
            .replace(
                ", \
            type2_schema_revision, type2_pre_activation)",
                ")",
            )
            .replace(", , false)", ")");
        let mut tx = pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "{mint}; \
             INSERT INTO game_item_decay_retire_reservations \
               (item_instance_id, fence_scope_ownership_generation, corpse_item_instance_id, \
                world_id, channel_id, transaction_id, event_id, quantity_before, \
                placement_ordinal, deadline, occurred_at, envelope, fence_holder_node_id, \
                fence_holder_registration_revision, work_units_used, reserved_at) \
             VALUES ({item}, 1, {corpse}, {world}, {channel}, {tx}, {ev}, 1, 1, 60000, 60000, \
                     decode(repeat('ab',16),'hex'), {node}, 0, 0, 900)",
            item = uuid(180),
            corpse = uuid(181),
            world = uuid(WORLD),
            channel = uuid(CHANNEL),
            tx = uuid(182),
            ev = uuid(183),
            node = uuid(1),
        )))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        database.migrate(None).await?;
        assert_eq!(marker_of(&pool, 170).await?, (None, true));
        let decay: (Option<i16>, bool) = sqlx::query_as(
            "SELECT type2_schema_revision, type2_pre_activation \
               FROM game_item_decay_retire_reservations",
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(decay, (None, true));
        // DUR-03's whole-row guard already refuses the change; with it set aside the marker
        // guard refuses it on its own.
        for change in ["type2_pre_activation = false", "type2_schema_revision = 2"] {
            let mut tx = pool.begin().await?;
            sqlx::query(
                "ALTER TABLE game_item_decay_retire_reservations \
                   DISABLE TRIGGER game_item_decay_retire_reservation_guard",
            )
            .execute(&mut *tx)
            .await?;
            let error = sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
                "UPDATE game_item_decay_retire_reservations SET {change}"
            )))
            .execute(&mut *tx)
            .await
            .expect_err("the marker is refused");
            tx.rollback().await?;
            assert_eq!(sqlstate(&error), "23514", "{change}");
            let message = error
                .as_database_error()
                .map(|error| error.message().to_owned());
            assert!(
                message
                    .as_deref()
                    .is_some_and(|message| message.contains("marker is immutable")),
                "{message:?}"
            );
        }
        let activated: i64 = sqlx::query_scalar("SELECT count(*) FROM game_type2_audit_activation")
            .fetch_one(&pool)
            .await?;
        assert_eq!(activated, 0, "0079 creates the table empty");

        pool.close().await;
        database.cleanup().await
    })
}
