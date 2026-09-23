#![allow(clippy::expect_used)]
// Dedicated PostgreSQL target for WP5 #414 Character authority. Ordinary
// workspace runs skip when the routed PostgreSQL service is absent; only
// configured PostgreSQL 17.6 runs count as qualification evidence.
extern crate self as oteryn_game_server;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
#[allow(dead_code, unused_imports)]
#[path = "../src/domain/mod.rs"]
pub mod domain;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;

use durability::DurabilityRoot;
use durability::character_authority::{BootstrapCommand, CharacterAuthorityError};
use durability::character_authority_audit as audit;
use durability::runtime_scope_assignment::{BootstrapSecret, LaunchBinding, NodeIncarnationProof};
use oteryn_game_server::character_recovery_fence::{
    CharacterRecoveryError, CharacterRecoveryFenceV1, CharacterRecoveryStore,
    recovery_record_digest,
};
use oteryn_game_server::domain::{AccountId, CharacterId, WorldId};
use prost::Message;
use sqlx::{Connection, Executor};

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

#[test]
fn registered_payload_encoding_matches_descriptor_shape() {
    let bytes = audit::encode_bootstrap(&id(1), &id(2), &id(3));
    let decoded = audit::CharacterAuthorityBootstrappedV1::decode(bytes.as_slice())
        .expect("registered payload decodes");
    assert_eq!(decoded.account_id, id(1));
    assert_eq!(decoded.character_id, id(2));
    assert_eq!(decoded.world_id, id(3));
    assert_eq!(decoded.character_revision, 1);
    assert_eq!(audit::EVENT_TYPE_CHARACTER_AUTHORITY_BOOTSTRAPPED, 1);
    assert_eq!(audit::EVENT_SCHEMA_REVISION, 1);
    assert_eq!(
        audit::RETENTION_PROFILE,
        "CHARACTER_AUTHORITY_DURABLE_AUDIT_RETENTION_V1"
    );
}

#[test]
fn authority_identifiers_are_distinct_uuidv7_types() {
    assert!(AccountId::from_bytes(id(1)).is_ok());
    assert!(CharacterId::from_bytes(id(2)).is_ok());
    assert!(WorldId::from_bytes(id(3)).is_ok());
}

#[test]
fn external_recovery_register_is_strict_and_fail_closed() {
    let directory = std::env::temp_dir().join(format!(
        "oteryn-character-recovery-integration-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir(&directory).expect("retained directory");
    let store =
        CharacterRecoveryStore::open(&directory, "character-primary", "game-ops").expect("store");
    assert!(matches!(
        store.seal_current(),
        Err(CharacterRecoveryError::Unavailable)
    ));
    drop(
        store
            .authorize_fresh_store(id(1), 100)
            .expect("explicit fresh authorization"),
    );
    drop(
        store
            .begin_recovery(1, id(2), 200)
            .expect("strict successor"),
    );
    drop(
        store
            .begin_recovery(1, id(2), 200)
            .expect("exact ambiguous reconciliation"),
    );
    assert!(matches!(
        store.begin_recovery(1, id(3), 200),
        Err(CharacterRecoveryError::Conflict)
    ));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn postgres_schema_enforces_atomic_immutable_first_slice() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(async {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("character_authority_postgres requires configured PostgreSQL 17.6");
        return;
    };
    let mut connection = sqlx::PgConnection::connect(&admin)
        .await
        .expect("connect PostgreSQL");
    let schema = format!("character_authority_{}", std::process::id());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&mut connection)
        .await
        .expect("schema");
    sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
        .execute(&mut connection)
        .await
        .expect("path");
    sqlx::raw_sql(include_str!("../migrations/0005_character_authority.sql"))
        .execute(&mut connection)
        .await
        .expect("migration");
    let retained = std::env::temp_dir().join(format!("oteryn-character-pg-fence-{schema}"));
    let _ = std::fs::remove_dir_all(&retained);
    std::fs::create_dir(&retained).expect("external retained directory");
    let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
        .expect("external recovery store");
    let fresh = recovery
        .authorize_fresh_store(id(11), 100)
        .expect("explicit fresh authorization");
    sqlx::query("INSERT INTO game_character_recovery_admissions(authority_scope_id, recovery_generation, recovery_event_id, predecessor_generation, issued_at, issuer_identity, reconciled_at) VALUES ('character-primary', 1, encode($1,'hex')::uuid, 0, 100, 'game-ops', 100)")
        .bind(id(11).as_slice())
        .execute(&mut connection)
        .await
        .expect("fresh DB admission");
    drop(fresh);
    let transition = recovery
        .begin_recovery(1, id(12), 200)
        .expect("external strict successor");
    let db_generation: String = sqlx::query_scalar("SELECT max(recovery_generation)::text FROM game_character_recovery_admissions")
        .fetch_one(&mut connection).await.expect("DB generation");
    assert_eq!(db_generation, "1");
    assert_eq!(transition.record.recovery_generation, 2, "older DB stays distinguishable while the external retained directory remains advanced");
    sqlx::query("INSERT INTO game_character_recovery_admissions(authority_scope_id, recovery_generation, recovery_event_id, predecessor_generation, predecessor_digest, issued_at, issuer_identity, reconciled_at) VALUES ('character-primary', 2, encode($1,'hex')::uuid, 1, $2, 200, 'game-ops', 200)")
        .bind(id(12).as_slice())
        .bind(transition.record.predecessor_digest.as_slice())
        .execute(&mut connection)
        .await
        .expect("explicit recovery reconciliation");
    drop(transition);
    let mut tx = connection.begin().await.expect("tx");
    sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES ('01890f4c-3b2a-7cc2-8d11-9a321b7c0001')").execute(&mut *tx).await.expect("guard");
    sqlx::query("INSERT INTO game_character_roots VALUES ('01890f4c-3b2a-7cc2-8d11-9a321b7c0002','01890f4c-3b2a-7cc2-8d11-9a321b7c0001','01890f4c-3b2a-7cc2-8d11-9a321b7c0003',1,1,'p','r','c','s')").execute(&mut *tx).await.expect("root");
    tx.rollback().await.expect("rollback");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_roots")
        .fetch_one(&mut connection)
        .await
        .expect("count");
    assert_eq!(count, 0, "rollback cannot leave an unaudited root");
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&mut connection)
        .await
        .expect("cleanup");
    std::fs::remove_dir_all(retained).expect("retained cleanup");
        });
}

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Database {
    admin_url: String,
    name: String,
    url: String,
}

impl Database {
    async fn create(admin_url: String, test_name: &str) -> TestResult<Self> {
        if !admin_url.starts_with("postgresql://oteryn_test_admin:")
            || !admin_url.ends_with("@127.0.0.1:5432/postgres")
        {
            return Err("unsafe PostgreSQL test admin URL".into());
        }
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = format!("ca_{test_name}_{suffix}");
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
        assert_eq!(
            version, "170006",
            "canonical target requires PostgreSQL 17.6"
        );
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

async fn register(
    root: &DurabilityRoot,
    tag: u8,
    supersedes: Option<u8>,
) -> TestResult<NodeIncarnationProof> {
    let secret = BootstrapSecret::from_bytes([tag; 32]);
    let launch = LaunchBinding::new(&format!("launch-{tag}")).map_err(|e| format!("{e:?}"))?;
    let node = foundation::NodeId::decode(&id(tag)).map_err(|e| format!("{e:?}"))?;
    let supersedes = supersedes
        .map(|previous| foundation::NodeId::decode(&id(previous)))
        .transpose()
        .map_err(|e| format!("{e:?}"))?;
    root.issue_node_bootstrap_authorization(&secret, &launch, supersedes)
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(root
        .register_node_incarnation(&secret, &launch, node)
        .await
        .map_err(|e| format!("{e:?}"))?)
}

fn command(operation: u8, account: u8) -> TestResult<BootstrapCommand> {
    Ok(BootstrapCommand {
        operation_id: id(operation),
        account_id: AccountId::from_bytes(id(account)).map_err(|e| format!("{e:?}"))?,
        world_id: WorldId::from_bytes(id(90)).map_err(|e| format!("{e:?}"))?,
        profile_revision: "profile-1".to_owned(),
        ruleset_revision: "ruleset-1".to_owned(),
        content_revision: "content-1".to_owned(),
        starter_template_revision: "starter-1".to_owned(),
    })
}

async fn count(pool: &sqlx::PgPool, sql: &'static str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(sql).fetch_one(pool).await?)
}

#[test]
fn bootstrap_is_atomic_and_audit_is_published_held_and_expired() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured");
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let database = Database::create(admin, "authority").await?;
            let result = bootstrap_audit_flow(&database).await;
            database.cleanup().await?;
            result
        })
}

async fn bootstrap_audit_flow(database: &Database) -> TestResult {
    let root = DurabilityRoot::connect_test_runtime(&database.url)?;
    assert!(root.maintain_ready_once().await?);
    let pool = sqlx::PgPool::connect(&database.url).await?;
    let retained = std::env::temp_dir().join(format!("oteryn-character-api-{}", database.name));
    let _ = std::fs::remove_dir_all(&retained);
    std::fs::create_dir(&retained)?;
    let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
        .map_err(|e| format!("{e:?}"))?;
    {
        let fresh = recovery
            .authorize_fresh_store(id(11), 100)
            .map_err(|e| format!("{e:?}"))?;
        root.admit_fresh_character_recovery(&fresh)
            .await
            .map_err(|e| format!("{e:?}"))?;
    }
    let fence = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    let authority = root
        .open_character_authority(&fence)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let node = register(&root, 1, None).await?;

    // Mutation, receipt, audit event and outbox commit together; a replay of the
    // exact operation returns the same stable identities, a changed one conflicts.
    let first = root
        .bootstrap_character(&authority, &node, command(21, 31)?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let replay = root
        .bootstrap_character(&authority, &node, command(21, 31)?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(replay, first);
    assert!(matches!(
        root.bootstrap_character(&authority, &node, command(21, 32)?)
            .await,
        Err(CharacterAuthorityError::Conflict)
    ));
    // Operation identities must be canonical UUIDv7 (version and RFC variant).
    let mut invalid = command(23, 34)?;
    invalid.operation_id[8] = 0x40;
    assert!(matches!(
        root.bootstrap_character(&authority, &node, invalid).await,
        Err(CharacterAuthorityError::Rejected)
    ));
    let decoded = audit::CharacterAuthorityBootstrappedV1::decode(
        first.payload.as_deref().ok_or("payload retained")?,
    )?;
    assert_eq!(decoded.account_id, id(31).to_vec());
    assert_eq!(decoded.character_id, first.character_id.as_bytes().to_vec());
    for (table, expected) in [
        ("SELECT count(*) FROM game_character_roots", 1),
        ("SELECT count(*) FROM game_character_operation_receipts", 1),
        ("SELECT count(*) FROM game_character_audit_outbox", 1),
    ] {
        assert_eq!(count(&pool, table).await?, expected, "{table}");
    }
    assert_eq!(
        root.read_current_character(&authority, first.character_id)
            .await
            .map_err(|e| format!("{e:?}"))?,
        first
    );

    // A superseded incarnation cannot mutate; its successor can.
    let successor = register(&root, 2, Some(1)).await?;
    let stale = root
        .bootstrap_character(&authority, &node, command(22, 33)?)
        .await;
    assert!(
        matches!(stale, Err(CharacterAuthorityError::Unavailable(_))),
        "{stale:?}"
    );
    assert_eq!(
        count(&pool, "SELECT count(*) FROM game_character_roots").await?,
        1
    );
    let node = successor;

    // At-least-once publication: pending until the exact acknowledgement,
    // and acknowledging again is a no-op.
    let pending = root
        .pending_character_audit(&authority, 8)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].event_id, first.event_id);
    assert_eq!(
        pending[0].server_build_id,
        durability::character_authority::SERVER_BUILD_ID
    );
    assert_eq!(Some(&pending[0].payload), first.payload.as_ref());
    // A pending event keeps its originating build across an upgrade: the
    // publisher reads the stored value, never the current process's build.
    let mut previous = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *previous)
        .await?;
    sqlx::query(
        "UPDATE game_character_audit_outbox SET server_build_id = 'oteryn-game-server/0.0.0-previous'",
    )
    .execute(&mut *previous)
    .await?;
    previous.commit().await?;
    assert!(
        sqlx::query("UPDATE game_character_audit_outbox SET server_build_id = 'rewritten'")
            .execute(&pool)
            .await
            .is_err()
    );
    let redelivered = root
        .pending_character_audit(&authority, 8)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        redelivered[0].server_build_id,
        "oteryn-game-server/0.0.0-previous"
    );
    let pending = redelivered;
    assert_eq!(
        root.pending_character_audit(&authority, 8)
            .await
            .map_err(|e| format!("{e:?}"))?,
        pending
    );
    for _ in 0..2 {
        root.acknowledge_character_audit(&authority, first.event_id)
            .await
            .map_err(|e| format!("{e:?}"))?;
    }
    assert!(
        root.pending_character_audit(&authority, 8)
            .await
            .map_err(|e| format!("{e:?}"))?
            .is_empty()
    );

    // Not yet expired: ordinary expiry deletes nothing and direct deletion fails.
    assert_eq!(
        root.expire_character_audit(&authority, 8)
            .await
            .map_err(|e| format!("{e:?}"))?,
        0
    );
    assert!(
        sqlx::query("DELETE FROM game_character_audit_outbox")
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE game_character_audit_outbox SET payload = '\\x00'")
            .execute(&pool)
            .await
            .is_err()
    );

    // Age the record past the P90D ceiling (test-only clock substitution).
    let mut aged = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *aged)
        .await?;
    sqlx::query("UPDATE game_character_audit_outbox SET occurred_at = occurred_at - 7776000001, expires_at = expires_at - 7776000001, published_at = occurred_at - 7776000001")
        .execute(&mut *aged)
        .await?;
    aged.commit().await?;

    // An explicit legal hold blocks ordinary expiry until its single release.
    let hold = root
        .place_character_audit_legal_hold(
            &authority,
            first.event_id,
            "case-1 investigation",
            "security:alice",
        )
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert!(matches!(
        root.place_character_audit_legal_hold(
            &authority,
            first.event_id,
            "duplicate",
            "security:alice"
        )
        .await,
        Err(CharacterAuthorityError::Conflict)
    ));
    assert_eq!(
        root.expire_character_audit(&authority, 8)
            .await
            .map_err(|e| format!("{e:?}"))?,
        0
    );
    assert!(
        sqlx::query("DELETE FROM game_character_audit_outbox")
            .execute(&pool)
            .await
            .is_err()
    );
    // TRUNCATE bypasses row triggers; every Character relation refuses it.
    for table in [
        "game_character_audit_legal_holds",
        "game_character_audit_outbox",
        "game_character_operation_receipts",
        "game_character_roots",
        "game_character_account_guards",
        "game_character_recovery_admissions",
    ] {
        assert!(
            sqlx::query(sqlx::AssertSqlSafe(format!("TRUNCATE {table} CASCADE")))
                .execute(&pool)
                .await
                .is_err(),
            "{table}"
        );
    }
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_character_audit_legal_holds WHERE released_at IS NULL"
        )
        .await?,
        1
    );
    root.release_character_audit_legal_hold(&authority, hold, "security:bob")
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert!(matches!(
        root.release_character_audit_legal_hold(&authority, hold, "security:bob")
            .await,
        Err(CharacterAuthorityError::Conflict)
    ));

    // Ordinary expiry deletes the player-linked event, envelope and payload;
    // authority state and its receipt remain, with no analytics copy.
    assert_eq!(
        root.expire_character_audit(&authority, 8)
            .await
            .map_err(|e| format!("{e:?}"))?,
        1
    );
    assert_eq!(
        count(&pool, "SELECT count(*) FROM game_character_audit_outbox").await?,
        0
    );
    let current = root
        .read_current_character(&authority, first.character_id)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(current.event_id, first.event_id);
    assert_eq!(current.payload, None);
    let replayed = root
        .bootstrap_character(&authority, &node, command(21, 31)?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(replayed.character_id, first.character_id);
    assert_eq!(replayed.event_id, first.event_id);
    assert_eq!(replayed.payload, None);
    let columns = count(
        &pool,
        "SELECT count(*) FROM information_schema.columns WHERE table_name = 'game_character_operation_receipts' AND column_name = 'payload'",
    )
    .await?;
    assert_eq!(columns, 0, "receipts retain no payload copy");

    // The capability is bound to the root whose database it checked.
    let other_root = DurabilityRoot::connect_test_runtime(&database.url)?;
    assert!(other_root.maintain_ready_once().await?);
    assert!(matches!(
        other_root.pending_character_audit(&authority, 8).await,
        Err(CharacterAuthorityError::Rejected)
    ));

    // Retained payloads are checked semantically, not only by their stored hash;
    // a receipt without its root is rejected.
    let second = root
        .bootstrap_character(&authority, &node, command(24, 35)?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let mut tamper = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tamper)
        .await?;
    sqlx::query("UPDATE game_character_audit_outbox SET payload = payload || '\\x00'::bytea, payload_sha256 = sha256(payload || '\\x00'::bytea) WHERE character_id = encode($1,'hex')::uuid")
        .bind(second.character_id.as_bytes().as_slice())
        .execute(&mut *tamper)
        .await?;
    tamper.commit().await?;
    let sealed = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    assert!(root.open_character_authority(&sealed).await.is_err());
    drop(sealed);
    let mut repair = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *repair)
        .await?;
    sqlx::query("UPDATE game_character_audit_outbox SET payload = substring(payload FROM 1 FOR octet_length(payload) - 1) WHERE character_id = encode($1,'hex')::uuid")
        .bind(second.character_id.as_bytes().as_slice())
        .execute(&mut *repair)
        .await?;
    sqlx::query("UPDATE game_character_audit_outbox SET payload_sha256 = sha256(payload) WHERE character_id = encode($1,'hex')::uuid")
        .bind(second.character_id.as_bytes().as_slice())
        .execute(&mut *repair)
        .await?;
    sqlx::query("INSERT INTO game_character_operation_receipts(operation_id, command_binding, account_id, character_id, world_id, character_revision, event_id, transaction_id) VALUES (encode($1,'hex')::uuid, '\\x01'::bytea, encode($2,'hex')::uuid, encode($3,'hex')::uuid, encode($2,'hex')::uuid, 1, encode($1,'hex')::uuid, encode($1,'hex')::uuid)")
        .bind(id(25).as_slice())
        .bind(id(36).as_slice())
        .bind(id(37).as_slice())
        .execute(&mut *repair)
        .await?;
    repair.commit().await?;
    let sealed = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    assert!(root.open_character_authority(&sealed).await.is_err());
    drop(sealed);
    let mut repair = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *repair)
        .await?;
    sqlx::query(
        "DELETE FROM game_character_operation_receipts WHERE operation_id = encode($1,'hex')::uuid",
    )
    .bind(id(25).as_slice())
    .execute(&mut *repair)
    .await?;
    repair.commit().await?;
    // Typed identities are canonical UUIDv7 in every column, even when row
    // triggers are bypassed: a version-7 id with a non-RFC variant is refused.
    let mut malformed = second.character_id.as_bytes().to_owned();
    malformed[8] = 0x40;
    for statement in [
        "UPDATE game_character_roots SET world_id = encode($1,'hex')::uuid",
        "UPDATE game_character_operation_receipts SET transaction_id = encode($1,'hex')::uuid",
        "UPDATE game_character_audit_outbox SET transaction_id = encode($1,'hex')::uuid",
    ] {
        let mut tamper = pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tamper)
            .await?;
        assert!(
            sqlx::query(statement)
                .bind(malformed.as_slice())
                .execute(&mut *tamper)
                .await
                .is_err(),
            "{statement}"
        );
        tamper.rollback().await?;
    }
    // An active legal hold whose audit event is missing is lost retention.
    sqlx::query("INSERT INTO game_character_audit_legal_holds(hold_id, event_id, reason, authorizing_actor, started_at) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 'case-2', 'security:alice', 1)")
        .bind(id(26).as_slice())
        .bind(id(27).as_slice())
        .execute(&pool)
        .await?;
    let sealed = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    assert!(root.open_character_authority(&sealed).await.is_err());
    drop(sealed);
    sqlx::query("UPDATE game_character_audit_legal_holds SET released_at = 2, released_by = 'security:bob' WHERE hold_id = encode($1,'hex')::uuid")
        .bind(id(26).as_slice())
        .execute(&pool)
        .await?;

    drop(authority);
    drop(fence);
    // An authority capability is issued only over an intact store: a receipt whose
    // command binding no longer reconstructs from its root fails closed.
    let mut tamper = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tamper)
        .await?;
    sqlx::query("UPDATE game_character_operation_receipts SET command_binding = '\\x00'::bytea || command_binding")
        .execute(&mut *tamper)
        .await?;
    tamper.commit().await?;
    let sealed = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    assert!(root.open_character_authority(&sealed).await.is_err());
    drop(sealed);
    let mut repair = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *repair)
        .await?;
    sqlx::query("UPDATE game_character_operation_receipts SET command_binding = substring(command_binding FROM 2)")
        .execute(&mut *repair)
        .await?;
    repair.commit().await?;
    let sealed = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    root.open_character_authority(&sealed)
        .await
        .map_err(|e| format!("{e:?}"))?;
    drop(sealed);

    // Another authority scope's successor cannot claim this database's predecessor.
    let foreign_dir =
        std::env::temp_dir().join(format!("oteryn-character-foreign-{}", database.name));
    let _ = std::fs::remove_dir_all(&foreign_dir);
    std::fs::create_dir(&foreign_dir)?;
    let foreign = CharacterRecoveryStore::open(&foreign_dir, "character-other", "game-ops")
        .map_err(|e| format!("{e:?}"))?;
    drop(
        foreign
            .authorize_fresh_store(id(14), 100)
            .map_err(|e| format!("{e:?}"))?,
    );
    let foreign_successor = foreign
        .begin_recovery(1, id(15), 400)
        .map_err(|e| format!("{e:?}"))?;
    assert!(
        root.reconcile_character_recovery(&foreign_successor)
            .await
            .is_err()
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_character_recovery_admissions"
        )
        .await?,
        1
    );
    drop(foreign_successor);
    std::fs::remove_dir_all(foreign_dir)?;

    // A malformed (non-UUIDv7) recovery event never advances the external register.
    let mut malformed = id(12);
    malformed[6] = 0x40;
    assert!(matches!(
        recovery.begin_recovery(1, malformed, 300),
        Err(CharacterRecoveryError::Rejected)
    ));
    assert_eq!(
        recovery
            .seal_current()
            .map_err(|e| format!("{e:?}"))?
            .record
            .recovery_generation,
        1
    );

    // Post-restore reconciliation refuses a receipt that contradicts its root.
    let first_attempt = recovery
        .begin_recovery(1, id(12), 300)
        .map_err(|e| format!("{e:?}"))?;
    drop(first_attempt);
    // An exact ambiguous re-run keeps the successor's retained predecessor
    // evidence, so a contradictory predecessor admission is refused.
    let transition = recovery
        .begin_recovery(1, id(12), 300)
        .map_err(|e| format!("{e:?}"))?;
    let generation_one = recovery_record_digest(&CharacterRecoveryFenceV1 {
        authority_scope_id: "character-primary".to_owned(),
        recovery_generation: 1,
        recovery_event_id: id(11),
        predecessor_generation: 0,
        predecessor_digest: [0; 32],
        issued_at: 100,
        issuer_identity: "game-ops".to_owned(),
    })
    .map_err(|e| format!("{e:?}"))?;
    assert_eq!(transition.record.predecessor_digest, generation_one);
    for issued_at in ["101", "100"] {
        let mut restore = pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *restore)
            .await?;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE game_character_recovery_admissions SET issued_at = {issued_at} WHERE recovery_generation = 1"
        )))
        .execute(&mut *restore)
        .await?;
        restore.commit().await?;
        if issued_at == "101" {
            assert!(
                root.reconcile_character_recovery(&transition)
                    .await
                    .is_err()
            );
        }
    }
    let mut tamper = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tamper)
        .await?;
    sqlx::query("UPDATE game_character_operation_receipts SET world_id = encode($1,'hex')::uuid")
        .bind(id(91).as_slice())
        .execute(&mut *tamper)
        .await?;
    tamper.commit().await?;
    assert!(
        root.reconcile_character_recovery(&transition)
            .await
            .is_err()
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_character_recovery_admissions"
        )
        .await?,
        1
    );
    let mut repair = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *repair)
        .await?;
    sqlx::query("UPDATE game_character_operation_receipts SET world_id = encode($1,'hex')::uuid")
        .bind(id(90).as_slice())
        .execute(&mut *repair)
        .await?;
    repair.commit().await?;
    root.reconcile_character_recovery(&transition)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_character_recovery_admissions"
        )
        .await?,
        2
    );
    drop(transition);
    let concurrent_fence = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    let roots_before: i64 = count(&pool, "SELECT count(*) FROM game_character_roots").await?;
    // Concurrent reuse of one operation identity with different accounts from two
    // independent roots yields one commit and one deterministic Conflict.
    let url = database.url.clone();
    let barrier = std::sync::Barrier::new(2);
    let outcomes: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = [41_u8, 42]
            .into_iter()
            .map(|account| {
                let (url, barrier, fence, node) = (&url, &barrier, &concurrent_fence, &node);
                scope.spawn(move || -> String {
                    let run = || -> TestResult<String> {
                        let runtime = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()?;
                        runtime.block_on(async {
                            let root = DurabilityRoot::connect_test_runtime(url)?;
                            assert!(root.maintain_ready_once().await?);
                            let authority = root
                                .open_character_authority(fence)
                                .await
                                .map_err(|e| format!("{e:?}"))?;
                            barrier.wait();
                            Ok(
                                match root
                                    .bootstrap_character(&authority, node, command(40, account)?)
                                    .await
                                {
                                    Ok(_) => "committed".to_owned(),
                                    Err(CharacterAuthorityError::Conflict) => "conflict".to_owned(),
                                    Err(error) => format!("{error:?}"),
                                },
                            )
                        })
                    };
                    run().unwrap_or_else(|error| format!("error: {error}"))
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap_or_else(|_| "panicked".to_owned()))
            .collect()
    });
    let mut sorted = outcomes.clone();
    sorted.sort();
    assert_eq!(sorted, ["committed", "conflict"], "{outcomes:?}");
    let roots_after: i64 = count(&pool, "SELECT count(*) FROM game_character_roots").await?;
    assert_eq!(roots_after, roots_before + 1);
    drop(concurrent_fence);

    pool.close().await;
    std::fs::remove_dir_all(retained)?;
    Ok(())
}

#[test]
fn fresh_store_admission_refuses_any_prior_character_row() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured");
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let database = Database::create(admin, "fresh").await?;
            let result = fresh_refusal(&database).await;
            database.cleanup().await?;
            result
        })
}

async fn fresh_refusal(database: &Database) -> TestResult {
    let root = DurabilityRoot::connect_test_runtime(&database.url)?;
    assert!(root.maintain_ready_once().await?);
    let pool = sqlx::PgPool::connect(&database.url).await?;
    // A dangling account guard alone is evidence of prior Character state.
    sqlx::query(
        "INSERT INTO game_character_account_guards(account_id) VALUES (encode($1,'hex')::uuid)",
    )
    .bind(id(31).as_slice())
    .execute(&pool)
    .await?;
    let retained = std::env::temp_dir().join(format!("oteryn-character-fresh-{}", database.name));
    let _ = std::fs::remove_dir_all(&retained);
    std::fs::create_dir(&retained)?;
    let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
        .map_err(|e| format!("{e:?}"))?;
    let fresh = recovery
        .authorize_fresh_store(id(11), 100)
        .map_err(|e| format!("{e:?}"))?;
    assert!(matches!(
        root.admit_fresh_character_recovery(&fresh).await,
        Err(CharacterAuthorityError::Conflict)
    ));
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_character_recovery_admissions"
        )
        .await?,
        0
    );
    drop(fresh);
    pool.close().await;
    std::fs::remove_dir_all(retained)?;
    Ok(())
}
