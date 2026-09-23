#![allow(clippy::expect_used)]
// Dedicated PostgreSQL target for WP5 #414 Character authority. Ordinary
// workspace runs skip when the routed PostgreSQL service is absent; only
// configured PostgreSQL 17.6 runs count as qualification evidence.
extern crate self as oteryn_game_server;
#[allow(dead_code, unused_imports)]
#[path = "../src/admission_evidence.rs"]
pub mod admission_evidence;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
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
#[allow(dead_code, unused_imports)]
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;

use durability::DurabilityRoot;
use durability::character_authority::CharacterAuthorityError;
use durability::character_authority_audit as audit;
use durability::native_admission_source::{
    DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
    SourceObservation,
};
use durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
    RuntimeScopeAssignmentWriter,
};
use durability::runtime_scope_assignment::{BootstrapSecret, LaunchBinding, NodeIncarnationProof};
use oteryn_game_server::character_bootstrap_intent::{
    CharacterBootstrapIntentV1, CharacterInterpretationV1, decode_producer_response,
};
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

/// Registered schema revision 1 payload bytes are pinned. Stored payloads are
/// retained only until expiry and later re-derived from authority state, so an
/// encoder change that alters these bytes must be a new schema revision.
#[test]
fn registered_payload_bytes_are_pinned_for_schema_revision_one() {
    let bytes = audit::encode_bootstrap(&id(1), &id(2), &id(3));
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        hex,
        "0a100102030405067008800a0b0c0d0e0f0112100202030405067008800a0b0c0d0e0f02\
         1a100302030405067008800a0b0c0d0e0f03200128013001"
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

fn uuid(bytes: [u8; 16]) -> String {
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn now_secs() -> TestResult<i64> {
    Ok(i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
    )?)
}

/// Exact Platform producer decision (`CHARACTER_AUTHENTICATED_BOOTSTRAP_INTENT_V1`).
#[derive(Clone)]
struct Wire {
    operation: u8,
    account: u8,
    world: u8,
    revision: i64,
    decision: i64,
    content: &'static str,
    variant: &'static str,
    issued: i64,
    expires: i64,
}

impl Wire {
    fn new(operation: u8, account: u8, revision: i64) -> TestResult<Self> {
        let now = now_secs()?;
        Ok(Self {
            operation,
            account,
            world: 90,
            revision,
            decision: revision,
            content: "content-1",
            variant: "OPERATOR_CONTROL_PLANE_BOOTSTRAP",
            issued: now - 1,
            expires: now + 120,
        })
    }

    fn json(&self) -> String {
        format!(
            r#"{{"contract_version":1,"variant":"{}","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"3f0c5b7e-1d2a-4c3b-9a8f-{:012x}","source_revision":"{}","operation_id":"{}","operation":"INITIAL_CHARACTER_BOOTSTRAP","account_id":"{}","target_world_id":"{}","interpretation_context":{{"profile_revision":"profile-1","ruleset_revision":"ruleset-1","content_revision":"{}","starter_template_revision":"starter-1"}},"issued_at_source":"{}","expires_at_source":"{}","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}}"#,
            self.variant,
            self.decision,
            self.revision,
            uuid(id(self.operation)),
            uuid(id(self.account)),
            uuid(id(self.world)),
            self.content,
            self.issued,
            self.expires
        )
    }

    fn decode(&self) -> TestResult<CharacterBootstrapIntentV1> {
        decode_producer_response(self.json().as_bytes(), id(self.operation))
            .map_err(|e| format!("{e:?}").into())
    }
}

/// Operator-only procedure: configure the Game-owned current interpretation.
async fn configure(pool: &sqlx::PgPool, value: [&str; 4]) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT game_character_configure_interpretation($1, $2, $3, $4)")
            .bind(value[0])
            .bind(value[1])
            .bind(value[2])
            .bind(value[3])
            .fetch_one(pool)
            .await?,
    )
}

/// Operator-only procedure: place an explicit legal hold.
async fn place_hold(
    pool: &sqlx::PgPool,
    event: [u8; 16],
    reason: &str,
    actor: &str,
) -> TestResult<[u8; 16]> {
    let hold: String = sqlx::query_scalar(
        "SELECT game_character_place_legal_hold(encode($1,'hex')::uuid, $2, $3)::text",
    )
    .bind(event.as_slice())
    .bind(reason)
    .bind(actor)
    .fetch_one(pool)
    .await?;
    let hex: String = hold.chars().filter(|c| *c != '-').collect();
    let mut out = [0u8; 16];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)?;
    }
    Ok(out)
}

/// Operator-only procedure: release an active legal hold once.
async fn release_hold(pool: &sqlx::PgPool, hold: [u8; 16], actor: &str) -> TestResult {
    sqlx::query("SELECT game_character_release_legal_hold(encode($1,'hex')::uuid, $2)")
        .bind(hold.as_slice())
        .bind(actor)
        .execute(pool)
        .await?;
    Ok(())
}

/// Game-owned current world evidence: assign one Channel of `world` (#415).
async fn assign_world(
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    world: u8,
    tag: u8,
) -> TestResult {
    let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "writer-a")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let scope = foundation::RuntimeScopeRefV1::channel(
        foundation::WorldId::decode(&id(world)).map_err(|e| format!("{e:?}"))?,
        foundation::ChannelId::decode(&id(tag)).map_err(|e| format!("{e:?}"))?,
    );
    let outcome = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([tag; 32]),
            actor: ControlActor::new("operator.control-plane").map_err(|e| format!("{e:?}"))?,
            command: AssignmentCommand::Assign {
                scope,
                target: node.fact(),
            },
        })
        .await
        .map_err(|e| format!("{e:?}"))?;
    if !matches!(outcome, AssignmentOutcome::Committed(_)) {
        return Err(format!("{outcome:?}").into());
    }
    Ok(())
}

fn intent(operation: u8, account: u8, revision: i64) -> TestResult<CharacterBootstrapIntentV1> {
    Wire::new(operation, account, revision)?.decode()
}

static S2_REVISION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

async fn initialize_s2(root: &DurabilityRoot, node: &NodeIncarnationProof) -> TestResult {
    root.initialize_native_admission_source(
        node,
        FreshStoreProvenance {
            namespace: "store:one".into(),
            authorization: "owner:approved".into(),
            source_authority: "platform".into(),
            initialized_at: 10,
        },
        DescriptorRegistration {
            revision: 1,
            facts: vec![1],
            installed_at: 10,
        },
    )
    .await
    .map_err(|e| format!("initialize S2: {e:?}"))?;
    Ok(())
}

/// Accept one exact authenticated S1 account-security observation into S2.
async fn observe_account(
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    account: u8,
    allowed: bool,
    observed_at: i64,
) -> TestResult {
    let revision = S2_REVISION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let account_id = uuid(id(account));
    let body = format!(
        r#"{{"version":1,"operation":"ReadAccountSecurityV1","result":"observed","source_authority":"platform","source_revision":"{revision}","decision_identity":"{revision}","source_observed_at":"{observed_at}","clock_uncertainty_seconds":"0","account_id":"{account_id}","purpose":"platform_security","scope":"fresh_admission","allowed":{allowed},"minimum_valid_generation":"1"}}"#
    );
    root.accept_native_source_observation(
        node,
        SourceObservation {
            source_authority: "platform".into(),
            operation: NativeSourceOperation::ReadAccountSecurityV1,
            subject: NativeSourceSubject::account_security(account_id)
                .map_err(|e| format!("{e:?}"))?,
            source_revision: revision,
            decision_identity: revision.to_string(),
            observed_at,
            semantic_facts: body.into_bytes(),
        },
    )
    .await
    .map_err(|e| format!("accept S2 observation: {e:?}"))?;
    Ok(())
}

async fn allow(root: &DurabilityRoot, node: &NodeIncarnationProof, account: u8) -> TestResult {
    observe_account(root, node, account, true, now_secs()?).await
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
    initialize_s2(&root, &node).await?;
    assign_world(&root, &node, 90, 95).await?;
    assert_eq!(
        configure(&pool, ["profile-1", "ruleset-1", "content-1", "starter-1"]).await?,
        1
    );

    // Mutation, receipt, audit event and outbox commit together; a replay of the
    // exact operation returns the same stable identities, a changed one conflicts.
    let first_intent = intent(21, 31, 1)?;
    allow(&root, &node, 31).await?;
    let first = root
        .bootstrap_character(&authority, &node, &first_intent)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let replay = root
        .bootstrap_character(&authority, &node, &first_intent)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(replay, first);
    assert!(matches!(
        root.bootstrap_character(&authority, &node, &intent(21, 32, 2)?)
            .await,
        Err(CharacterAuthorityError::Conflict)
    ));
    // Operation identities must be canonical UUIDv7 (version and RFC variant).
    let mut operation = id(23);
    operation[8] = 0x40;
    let raw = Wire::new(23, 34, 2)?
        .json()
        .replace(&uuid(id(23)), &uuid(operation));
    assert!(decode_producer_response(raw.as_bytes(), operation).is_err());
    let decoded = audit::CharacterAuthorityBootstrappedV1::decode(first.payload.as_slice())?;
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
    allow(&root, &node, 33).await?;
    let successor = register(&root, 2, Some(1)).await?;
    let stale = root
        .bootstrap_character(&authority, &node, &intent(22, 33, 3)?)
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
    root.claim_native_admission_source_custody(&node)
        .await
        .map_err(|e| format!("{e:?}"))?;

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
    assert_eq!(pending[0].payload, first.payload);
    // A pending event keeps its originating build across an upgrade: the
    // publisher reads the stored value, never the current process's build.
    let mut previous = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *previous)
        .await?;
    for table in [
        "game_character_audit_outbox",
        "game_character_operation_receipts",
    ] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE {table} SET server_build_id = 'oteryn-game-server/0.0.0-previous'"
        )))
        .execute(&mut *previous)
        .await?;
    }
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
    // Shifting the envelope's occurrence time (and its retention deadline)
    // contradicts the durable receipt, so authority is refused.
    for shift in ["- 1000", "+ 1000"] {
        let mut shifted = pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *shifted)
            .await?;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE game_character_audit_outbox SET occurred_at = occurred_at {shift}, expires_at = expires_at {shift}"
        )))
        .execute(&mut *shifted)
        .await?;
        shifted.commit().await?;
        if shift == "- 1000" {
            let sealed = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
            assert!(root.open_character_authority(&sealed).await.is_err());
            drop(sealed);
        }
    }
    // Substituting the retained envelope's build alone contradicts the durable
    // receipt binding, so authority is refused until it is repaired.
    let mut substitute = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *substitute)
        .await?;
    sqlx::query("UPDATE game_character_audit_outbox SET server_build_id = 'substituted-build'")
        .execute(&mut *substitute)
        .await?;
    substitute.commit().await?;
    let sealed = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    assert!(root.open_character_authority(&sealed).await.is_err());
    drop(sealed);
    let mut repair = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *repair)
        .await?;
    sqlx::query(
        "UPDATE game_character_audit_outbox SET server_build_id = 'oteryn-game-server/0.0.0-previous'",
    )
    .execute(&mut *repair)
    .await?;
    repair.commit().await?;
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
    sqlx::query(
        "UPDATE game_character_operation_receipts SET occurred_at = occurred_at - 7776000001",
    )
    .execute(&mut *aged)
    .await?;
    aged.commit().await?;

    // Database boundary: a direct DELETE that starts while a hold is being
    // placed waits on the retention lock and then sees the committed hold.
    let mut holding = pool.begin().await?;
    sqlx::query("INSERT INTO game_character_audit_legal_holds(hold_id, event_id, reason, authorizing_actor, started_at) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 'case-race', 'security:alice', 1)")
        .bind(id(29).as_slice())
        .bind(first.event_id.as_slice())
        .execute(&mut *holding)
        .await?;
    let racer = pool.clone();
    let delete = tokio::spawn(async move {
        sqlx::query("DELETE FROM game_character_audit_outbox")
            .execute(&racer)
            .await
            .is_err()
    });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    holding.commit().await?;
    assert!(
        delete.await?,
        "direct delete removed an event held concurrently"
    );
    assert_eq!(
        count(&pool, "SELECT count(*) FROM game_character_audit_outbox").await?,
        1
    );
    sqlx::query("UPDATE game_character_audit_legal_holds SET released_at = 2, released_by = 'security:bob' WHERE hold_id = encode($1,'hex')::uuid")
        .bind(id(29).as_slice())
        .execute(&pool)
        .await?;

    // An explicit legal hold blocks ordinary expiry until its single release.
    // Holds are operator-only procedures, not a Game server API.
    let hold = place_hold(
        &pool,
        first.event_id,
        "case-1 investigation",
        "security:alice",
    )
    .await?;
    // An exact replay after a lost response returns the committed hold, so it
    // stays releasable; a different placement on the same event conflicts.
    assert_eq!(
        place_hold(
            &pool,
            first.event_id,
            "case-1 investigation",
            "security:alice"
        )
        .await?,
        hold
    );
    assert!(
        place_hold(&pool, first.event_id, "duplicate", "security:alice")
            .await
            .is_err()
    );
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
    release_hold(&pool, hold, "security:bob").await?;
    assert!(release_hold(&pool, hold, "security:bob").await.is_err());

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
    // Payload bytes are re-derived from retained authority after expiry.
    assert_eq!(current.payload, first.payload);
    let replayed = root
        .bootstrap_character(&authority, &node, &first_intent)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(replayed.character_id, first.character_id);
    assert_eq!(replayed.event_id, first.event_id);
    assert_eq!(replayed.payload, first.payload);
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
    allow(&root, &node, 35).await?;
    let second = root
        .bootstrap_character(&authority, &node, &intent(24, 35, 4)?)
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
    sqlx::query("INSERT INTO game_character_operation_receipts(operation_id, command_binding, account_id, character_id, world_id, character_revision, event_id, transaction_id, server_build_id, occurred_at, issuer_decision_id, intent_source_revision, issued_at_source, expires_at_source) VALUES (encode($1,'hex')::uuid, '\\x01'::bytea, encode($2,'hex')::uuid, encode($3,'hex')::uuid, encode($2,'hex')::uuid, 1, encode($1,'hex')::uuid, encode($1,'hex')::uuid, 'orphan-build', 1, encode($1,'hex')::uuid, 1000, 1, 2)")
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
    let reuse = [intent(40, 41, 5)?, intent(40, 42, 6)?];
    allow(&root, &node, 41).await?;
    allow(&root, &node, 42).await?;
    // Concurrent reuse of one operation identity with different accounts from two
    // independent roots yields one commit and one deterministic Conflict.
    let url = database.url.clone();
    let barrier = std::sync::Barrier::new(2);
    let outcomes: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = reuse
            .iter()
            .map(|reused| {
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
                                match root.bootstrap_character(&authority, node, reused).await {
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

async fn totals(pool: &sqlx::PgPool) -> TestResult<[i64; 5]> {
    Ok([
        count(pool, "SELECT count(*) FROM game_character_account_guards").await?,
        count(pool, "SELECT count(*) FROM game_character_roots").await?,
        count(
            pool,
            "SELECT count(*) FROM game_character_operation_receipts",
        )
        .await?,
        count(pool, "SELECT count(*) FROM game_character_audit_outbox").await?,
        count(
            pool,
            "SELECT count(*) FROM game_character_bootstrap_intent_floors",
        )
        .await?,
    ])
}

async fn floor(pool: &sqlx::PgPool) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT source_revision FROM game_character_bootstrap_intent_floors WHERE issuer_scope = 1",
    )
    .fetch_one(pool)
    .await?)
}

/// Run each intent from an independent root on its own thread at one barrier.
fn concurrently(
    url: &str,
    fence: &oteryn_game_server::character_recovery_fence::SealedCharacterRecoveryFence<'_>,
    node: &NodeIncarnationProof,
    intents: &[CharacterBootstrapIntentV1],
) -> Vec<Result<durability::character_authority::CharacterAuthorityRecord, String>> {
    let barrier = std::sync::Barrier::new(intents.len());
    std::thread::scope(|scope| {
        let handles: Vec<_> = intents
            .iter()
            .map(|intent| {
                let barrier = &barrier;
                scope.spawn(move || {
                    let run = || -> TestResult<Result<_, String>> {
                        tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()?
                            .block_on(async {
                                let root = DurabilityRoot::connect_test_runtime(url)?;
                                assert!(root.maintain_ready_once().await?);
                                let authority = root
                                    .open_character_authority(fence)
                                    .await
                                    .map_err(|e| format!("{e:?}"))?;
                                barrier.wait();
                                Ok(root
                                    .bootstrap_character(&authority, node, intent)
                                    .await
                                    .map_err(|e| format!("{e:?}")))
                            })
                    };
                    run().unwrap_or_else(|error| Err(format!("error: {error}")))
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap_or_else(|_| Err("panicked".to_owned())))
            .collect()
    })
}

#[test]
fn authenticated_intent_qualification_matrix() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured");
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let database = Database::create(admin, "intent").await?;
            let result = intent_matrix(&database).await;
            database.cleanup().await?;
            result
        })
}

async fn intent_matrix(database: &Database) -> TestResult {
    let root = DurabilityRoot::connect_test_runtime(&database.url)?;
    assert!(root.maintain_ready_once().await?);
    let pool = sqlx::PgPool::connect(&database.url).await?;
    let retained = std::env::temp_dir().join(format!("oteryn-character-intent-{}", database.name));
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
    initialize_s2(&root, &node).await?;
    let rejected = |result: Result<_, CharacterAuthorityError>| {
        matches!(result, Err(CharacterAuthorityError::Rejected))
    };

    // Neither a caller-filled command nor another variant is an intent.
    let caller_filled = format!(
        r#"{{"account_id":"{}","target_world_id":"{}"}}"#,
        uuid(id(70)),
        uuid(id(90))
    );
    assert!(decode_producer_response(caller_filled.as_bytes(), id(60)).is_err());
    let mut user_create = Wire::new(60, 70, 10)?;
    user_create.variant = "PLATFORM_USER_CREATE";
    assert!(user_create.decode().is_err());

    // Current allowed S1/S2 account security is an independent prerequisite:
    // absent, denied or stale evidence rejects with zero authoritative writes.
    let valid = intent(60, 70, 10)?;
    assert!(rejected(
        root.bootstrap_character(&authority, &node, &valid).await
    ));
    observe_account(&root, &node, 70, false, now_secs()?).await?;
    assert!(rejected(
        root.bootstrap_character(&authority, &node, &valid).await
    ));
    observe_account(&root, &node, 70, true, now_secs()? - 10).await?;
    assert!(rejected(
        root.bootstrap_character(&authority, &node, &valid).await
    ));
    assert_eq!(totals(&pool).await?, [0; 5]);

    // World and interpretation are requested context: a world with no currently
    // assigned Channel, no configured Game-owned interpretation, or a different
    // one rejects; the intent's revisions alone never authorize.
    allow(&root, &node, 70).await?;
    assert!(rejected(
        root.bootstrap_character(&authority, &node, &valid).await
    ));
    assign_world(&root, &node, 90, 95).await?;
    assert!(rejected(
        root.bootstrap_character(&authority, &node, &valid).await
    ));
    let other_content =
        CharacterInterpretationV1::new("profile-1", "ruleset-1", "content-2", "starter-1")
            .map_err(|e| format!("{e:?}"))?;
    let other_content = other_content.revisions();
    for expected in [1, 1] {
        assert_eq!(
            configure(&pool, other_content).await?,
            expected,
            "configuring the current value again is idempotent"
        );
    }
    assert!(rejected(
        root.bootstrap_character(&authority, &node, &valid).await
    ));
    assert_eq!(
        configure(&pool, ["profile-1", "ruleset-1", "content-1", "starter-1"]).await?,
        2
    );
    // The operator procedures are not executable by an unprivileged role such
    // as a Game server role (EXECUTE is revoked from PUBLIC).
    let mut unprivileged = pool.begin().await?;
    sqlx::query("CREATE ROLE character_server_probe NOLOGIN")
        .execute(&mut *unprivileged)
        .await?;
    sqlx::query("SET LOCAL ROLE character_server_probe")
        .execute(&mut *unprivileged)
        .await?;
    for procedure in [
        "SELECT game_character_configure_interpretation('a', 'b', 'c', 'd')",
        "SELECT game_character_release_legal_hold(game_character_uuid_v7(), 'x')",
    ] {
        let mut savepoint = unprivileged.begin().await?;
        assert!(
            sqlx::query(procedure)
                .execute(&mut *savepoint)
                .await
                .is_err(),
            "{procedure}"
        );
        savepoint.rollback().await?;
    }
    unprivileged.rollback().await?;
    // Deployment model: an operator role holding only EXECUTE acts through the
    // SECURITY DEFINER procedures but cannot write the tables directly, and
    // the revision grammar holds at the SQL boundary.
    let mut operator = pool.begin().await?;
    sqlx::query("CREATE ROLE character_operator_probe NOLOGIN")
        .execute(&mut *operator)
        .await?;
    sqlx::query("GRANT EXECUTE ON FUNCTION game_character_configure_interpretation(text, text, text, text), game_character_place_legal_hold(uuid, text, text), game_character_release_legal_hold(uuid, text) TO character_operator_probe")
        .execute(&mut *operator)
        .await?;
    sqlx::query("SET LOCAL ROLE character_operator_probe")
        .execute(&mut *operator)
        .await?;
    let revision: i64 = sqlx::query_scalar(
        "SELECT game_character_configure_interpretation('profile-9', 'ruleset-9', 'content-9', 'starter-9')",
    )
    .fetch_one(&mut *operator)
    .await?;
    assert_eq!(revision, 3, "appended after the two committed revisions");
    for statement in [
        "INSERT INTO game_character_interpretations VALUES (9, 'p', 'r', 'c', 's', 0)",
        "SELECT game_character_configure_interpretation('-profile', 'r', 'c', 's')",
        "SELECT game_character_configure_interpretation('profilé', 'r', 'c', 's')",
    ] {
        let mut savepoint = operator.begin().await?;
        assert!(
            sqlx::query(statement)
                .execute(&mut *savepoint)
                .await
                .is_err(),
            "{statement}"
        );
        savepoint.rollback().await?;
    }
    operator.rollback().await?;
    assert!(
        sqlx::query("UPDATE game_character_interpretations SET content_revision = 'content-3'")
            .execute(&pool)
            .await
            .is_err(),
        "interpretation history is append-only"
    );
    assert_eq!(totals(&pool).await?, [0; 5]);

    // Valid intent + current security + process proof + recovery fence: exactly
    // one Character, revision, receipt, audit event, outbox row and floor.
    allow(&root, &node, 70).await?;
    let first = root
        .bootstrap_character(&authority, &node, &valid)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(totals(&pool).await?, [1, 1, 1, 1, 1]);
    assert_eq!(floor(&pool).await?, 10);
    let binding: Vec<u8> = sqlx::query_scalar(
        "SELECT command_binding FROM game_character_operation_receipts WHERE operation_id = encode($1,'hex')::uuid",
    )
    .bind(id(60).as_slice())
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        binding,
        valid.binding(),
        "receipt binds the complete intent"
    );

    // Exact retry before or after a lost response returns the same result;
    // reconciliation by operation identity alone never re-authorizes.
    assert_eq!(
        root.bootstrap_character(&authority, &node, &valid)
            .await
            .map_err(|e| format!("{e:?}"))?,
        first
    );
    assert_eq!(
        root.reconcile_character_bootstrap(&authority, id(60))
            .await
            .map_err(|e| format!("{e:?}"))?,
        Some(first.clone())
    );
    assert_eq!(
        root.reconcile_character_bootstrap(&authority, id(99))
            .await
            .map_err(|e| format!("{e:?}"))?,
        None
    );

    // Same operation with changed AccountId, world or interpretation conflicts.
    let mut changed_world = Wire::new(60, 70, 11)?;
    changed_world.world = 91;
    let mut changed_context = Wire::new(60, 70, 11)?;
    changed_context.content = "content-2";
    for changed in [
        intent(60, 71, 11)?,
        changed_world.decode()?,
        changed_context.decode()?,
    ] {
        assert!(matches!(
            root.bootstrap_character(&authority, &node, &changed).await,
            Err(CharacterAuthorityError::Conflict)
        ));
    }

    // A lower source revision is stale; an equal one naming another decision
    // is a contradiction; expired and future intents are rejected.
    allow(&root, &node, 72).await?;
    let mut contradiction = Wire::new(62, 72, 10)?;
    contradiction.decision = 99;
    let mut expired = Wire::new(63, 72, 12)?;
    (expired.issued, expired.expires) = (now_secs()? - 200, now_secs()? - 1);
    let mut future = Wire::new(63, 72, 12)?;
    (future.issued, future.expires) = (now_secs()? + 30, now_secs()? + 60);
    for refused in [
        intent(61, 72, 9)?,
        contradiction.decode()?,
        expired.decode()?,
        future.decode()?,
    ] {
        assert!(rejected(
            root.bootstrap_character(&authority, &node, &refused).await
        ));
    }
    assert_eq!(totals(&pool).await?, [1, 1, 1, 1, 1]);
    assert_eq!(floor(&pool).await?, 10);

    // An audit/outbox failure rolls back the whole transaction, including the
    // intent high-water; the same intent then commits normally.
    sqlx::raw_sql(
        "CREATE FUNCTION test_fail_outbox() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected'; END; $$; \
         CREATE TRIGGER test_fail_outbox BEFORE INSERT ON game_character_audit_outbox FOR EACH ROW EXECUTE FUNCTION test_fail_outbox();",
    )
    .execute(&pool)
    .await?;
    let after_failure = intent(64, 72, 13)?;
    assert!(matches!(
        root.bootstrap_character(&authority, &node, &after_failure)
            .await,
        Err(CharacterAuthorityError::Unavailable(_))
    ));
    assert_eq!(totals(&pool).await?, [1, 1, 1, 1, 1]);
    assert_eq!(floor(&pool).await?, 10);
    sqlx::raw_sql("DROP TRIGGER test_fail_outbox ON game_character_audit_outbox; DROP FUNCTION test_fail_outbox();")
        .execute(&pool)
        .await?;
    allow(&root, &node, 72).await?;
    root.bootstrap_character(&authority, &node, &after_failure)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(floor(&pool).await?, 13);

    // Concurrent exact retries yield one semantic result.
    let retried = intent(65, 73, 14)?;
    allow(&root, &node, 73).await?;
    let outcomes = concurrently(&database.url, &fence, &node, &[retried.clone(), retried]);
    let [Ok(left), Ok(right)] = &outcomes[..] else {
        return Err(format!("{outcomes:?}").into());
    };
    assert_eq!(left, right);
    assert_eq!(totals(&pool).await?, [3, 3, 3, 3, 1]);

    // Concurrent distinct operations serialize: each commits or is refused as
    // stale by the source high-water, never duplicated or half-written.
    allow(&root, &node, 74).await?;
    allow(&root, &node, 75).await?;
    let outcomes = concurrently(
        &database.url,
        &fence,
        &node,
        &[intent(66, 74, 15)?, intent(67, 75, 16)?],
    );
    let committed = outcomes.iter().filter(|outcome| outcome.is_ok()).count();
    assert!(
        committed >= 1
            && outcomes.iter().all(|outcome| outcome.as_ref().is_ok()
                || outcome.as_ref().err().map(String::as_str) == Some("Rejected")),
        "{outcomes:?}"
    );
    let roots = i64::try_from(committed)? + 3;
    assert_eq!(totals(&pool).await?[1..4], [roots, roots, roots]);
    assert_eq!(floor(&pool).await?, 16);

    // Restart: a new root keeps the exact receipt and floor without re-authorizing.
    let restarted = DurabilityRoot::connect_test_runtime(&database.url)?;
    assert!(restarted.maintain_ready_once().await?);
    let reopened = restarted
        .open_character_authority(&fence)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        restarted
            .reconcile_character_bootstrap(&reopened, id(60))
            .await
            .map_err(|e| format!("{e:?}"))?,
        Some(first)
    );
    assert_eq!(floor(&pool).await?, 16);

    // A replaced #415 process proof is refused independently of the intent.
    allow(&root, &node, 76).await?;
    let _successor = register(&root, 2, Some(1)).await?;
    assert!(matches!(
        root.bootstrap_character(&authority, &node, &intent(68, 76, 17)?)
            .await,
        Err(CharacterAuthorityError::Unavailable(_))
    ));
    assert_eq!(floor(&pool).await?, 16);

    // A restore that drops or regresses the retained floor keeps bootstrap closed.
    drop(reopened);
    drop(authority);
    for tamper in [
        "DELETE FROM game_character_bootstrap_intent_floors",
        "UPDATE game_character_bootstrap_intent_floors SET source_revision = 15",
    ] {
        let mut restore = pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *restore)
            .await?;
        let kept: (i64, Vec<u8>, Vec<u8>) = sqlx::query_as(
            "SELECT source_revision, uuid_send(issuer_decision_id), intent_binding FROM game_character_bootstrap_intent_floors",
        )
        .fetch_one(&mut *restore)
        .await?;
        sqlx::query(sqlx::AssertSqlSafe(tamper.to_owned()))
            .execute(&mut *restore)
            .await?;
        restore.commit().await?;
        assert!(
            root.open_character_authority(&fence).await.is_err(),
            "{tamper}"
        );
        let mut repair = pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *repair)
            .await?;
        sqlx::query("DELETE FROM game_character_bootstrap_intent_floors")
            .execute(&mut *repair)
            .await?;
        sqlx::query("INSERT INTO game_character_bootstrap_intent_floors VALUES (1, $1, encode($2,'hex')::uuid, $3)")
            .bind(kept.0)
            .bind(kept.1)
            .bind(kept.2)
            .execute(&mut *repair)
            .await?;
        repair.commit().await?;
        root.open_character_authority(&fence)
            .await
            .map_err(|e| format!("{e:?}"))?;
    }
    // The high-water only advances, even for a direct statement.
    assert!(
        sqlx::query("UPDATE game_character_bootstrap_intent_floors SET source_revision = 1")
            .execute(&pool)
            .await
            .is_err()
    );

    drop(fence);
    pool.close().await;
    std::fs::remove_dir_all(retained)?;
    Ok(())
}
