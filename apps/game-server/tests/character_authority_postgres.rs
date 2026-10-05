#![allow(clippy::expect_used)]
// Dedicated PostgreSQL target for WP5 #414 Character authority. Ordinary
// workspace runs skip when the routed PostgreSQL service is absent; only
// configured PostgreSQL 17.6 runs count as qualification evidence.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;
pub use production_server::admission_evidence;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
pub use production_server::domain;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[allow(dead_code, unused_imports)]
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;
#[allow(dead_code, unused_imports)]
#[path = "../src/premium/mod.rs"]
mod premium;

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

/// A private parent for recovery-fence directories: the fence requires a
/// parent that is not writable by group or others (OPS-NODE-BOOT-01 D1).
fn fence_parent() -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let parent =
        std::env::temp_dir().join(format!("oteryn-character-fences-{}", std::process::id()));
    std::fs::create_dir_all(&parent).expect("fence parent");
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700))
        .expect("fence parent mode");
    parent
}

#[test]
fn external_recovery_register_is_strict_and_fail_closed() {
    let directory = fence_parent().join(format!(
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
    let retained = fence_parent().join(format!("oteryn-character-pg-fence-{schema}"));
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
    assert_eq!(transition.record().recovery_generation, 2, "older DB stays distinguishable while the external retained directory remains advanced");
    sqlx::query("INSERT INTO game_character_recovery_admissions(authority_scope_id, recovery_generation, recovery_event_id, predecessor_generation, predecessor_digest, issued_at, issuer_identity, reconciled_at) VALUES ('character-primary', 2, encode($1,'hex')::uuid, 1, $2, 200, 'game-ops', 200)")
        .bind(id(12).as_slice())
        .bind(transition.record().predecessor_digest.as_slice())
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
        Self::create_at(admin_url, test_name, None).await
    }

    /// `through`: apply migrations only up to and including this version.
    async fn create_at(
        admin_url: String,
        test_name: &str,
        through: Option<i64>,
    ) -> TestResult<Self> {
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
        let migrator = sqlx::migrate!("./migrations");
        match through {
            Some(version) => migrator.run_to(version, &mut connection).await?,
            None => migrator.run(&mut connection).await?,
        }
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
    name: String,
    issued: i64,
    expires: i64,
}

/// A distinct valid name (naming policy revision 1) per operation seed.
fn name_for(operation: u8) -> String {
    format!(
        "Hero {}{}",
        char::from(b'A' + operation / 26),
        char::from(b'a' + operation % 26)
    )
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
            name: name_for(operation),
            issued: now - 1,
            expires: now + 120,
        })
    }

    fn json(&self) -> String {
        format!(
            r#"{{"contract_version":2,"variant":"{}","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"3f0c5b7e-1d2a-4c3b-9a8f-{:012x}","source_revision":"{}","operation_id":"{}","operation":"INITIAL_CHARACTER_BOOTSTRAP","account_id":"{}","target_world_id":"{}","requested_name":"{}","interpretation_context":{{"profile_revision":"profile-1","ruleset_revision":"ruleset-1","content_revision":"{}","starter_template_revision":"starter-1"}},"issued_at_source":"{}","expires_at_source":"{}","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}}"#,
            self.variant,
            self.decision,
            self.revision,
            uuid(id(self.operation)),
            uuid(id(self.account)),
            uuid(id(self.world)),
            self.name,
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
    pool: &sqlx::PgPool,
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
    // Owner-written exact-scope grant for the session role, which is also
    // the recorded actor (OPS-NODE-BOOT-01 D2).
    sqlx::query(
        "INSERT INTO game_control_scope_grants (control_role, world_id, channel_id, operation) \
         VALUES (session_user, encode($1, 'hex')::uuid, encode($2, 'hex')::uuid, 1) ON CONFLICT DO NOTHING",
    )
    .bind(id(world).as_slice())
    .bind(id(tag).as_slice())
    .execute(pool)
    .await?;
    let outcome = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([tag; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(|e| format!("{e:?}"))?,
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
    let provenance = FreshStoreProvenance {
        namespace: "store:one".into(),
        authorization: "owner:approved".into(),
        source_authority: "platform".into(),
        initialized_at: 10,
    };
    let descriptor = DescriptorRegistration {
        revision: 1,
        facts: vec![1],
        installed_at: 10,
    };
    // The control-plane issuance precedes initialization (OPS-NODE-BOOT-01 D2).
    if !root
        .record_native_source_descriptor_issuance(
            "platform",
            descriptor.clone(),
            Some(provenance.clone()),
        )
        .await
        .map_err(|e| format!("S2 issuance: {e:?}"))?
    {
        return Err("S2 issuance refused".into());
    }
    root.initialize_native_admission_source(node, provenance, descriptor)
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
    let retained = fence_parent().join(format!("oteryn-character-api-{}", database.name));
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
    assign_world(&pool, &root, &node, 90, 95).await?;
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

    // LCFA-1: the bootstrap queued its account's snapshot in the same
    // transaction; the publisher reads it, clears it on acknowledgement, and
    // the watermark then has no undelivered change.
    let snapshot = root
        .next_account_characters_snapshot(&authority)
        .await
        .map_err(|e| format!("{e:?}"))?
        .ok_or("bootstrap queued no projection snapshot")?;
    assert_eq!(snapshot.account_id, uuid(id(31)));
    assert_eq!(
        (snapshot.projection_epoch, snapshot.projection_revision),
        (1, 1)
    );
    assert_eq!(
        snapshot.characters,
        vec![
            oteryn_game_server::native_admission_source::account_characters::CharacterSummary {
                character_id: uuid(*first.character_id.as_bytes()),
                world_id: uuid(id(90)),
                name: name_for(21),
                availability:
                    oteryn_game_server::native_admission_source::account_characters::Availability::Available,
            }
        ]
    );
    let body = oteryn_game_server::native_admission_source::account_characters::encode_snapshot(
        "oteryn:character-authority:primary",
        &snapshot,
    )?;
    // A retry of the same (epoch, revision) is byte-identical, however much
    // later it reads: `source_observed_at` is when the revision was assigned
    // (0028), so a lost acknowledgement never becomes a 409.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let retry = root
        .next_account_characters_snapshot(&authority)
        .await
        .map_err(|e| format!("{e:?}"))?
        .ok_or("the unacknowledged snapshot is still queued")?;
    assert_eq!(
        oteryn_game_server::native_admission_source::account_characters::encode_snapshot(
            "oteryn:character-authority:primary",
            &retry,
        )?,
        body
    );
    let facts = root
        .account_characters_watermark_facts(&authority)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(facts.projection_epoch, 1);
    // Never later than the read.
    assert!(retry.source_observed_at * 1000 <= facts.now_ms);
    assert!(
        facts
            .oldest_undelivered_ms
            .is_some_and(|oldest| oldest <= facts.now_ms)
    );
    root.clear_account_characters(&authority, &snapshot.account_id, 1, 1)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert!(
        root.next_account_characters_snapshot(&authority)
            .await
            .map_err(|e| format!("{e:?}"))?
            .is_none()
    );
    let facts = root
        .account_characters_watermark_facts(&authority)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(facts.oldest_undelivered_ms, None);

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
    // The refused (fenced) mutation queued no projection change either.
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_character_account_projection_outbox"
        )
        .await?,
        0
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
    // A temporary relation cannot shadow the holds table inside the guard: the
    // expired but held event still refuses direct deletion.
    let mut shadow = pool.begin().await?;
    sqlx::query("CREATE TEMP TABLE game_character_audit_legal_holds (event_id UUID, released_at BIGINT) ON COMMIT DROP")
        .execute(&mut *shadow)
        .await?;
    assert!(
        sqlx::query("DELETE FROM game_character_audit_outbox")
            .execute(&mut *shadow)
            .await
            .is_err()
    );
    shadow.rollback().await?;
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
    // Root revisions keep the interpretation grammar even when row triggers are
    // bypassed, so reconciliation never reopens unrepresentable context.
    let mut tamper = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tamper)
        .await?;
    assert!(
        sqlx::query("UPDATE game_character_roots SET profile_revision = '-profile'")
            .execute(&mut *tamper)
            .await
            .is_err()
    );
    tamper.rollback().await?;
    // Restored issuer decision identities keep the decoder's UUID semantics.
    for statement in [
        "UPDATE game_character_operation_receipts SET issuer_decision_id = '00000000-0000-0000-0000-000000000000'",
        "UPDATE game_character_bootstrap_intent_floors SET issuer_decision_id = '3f0c5b7e-1d2a-4c3b-4a8f-000000000001'",
    ] {
        let mut tamper = pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tamper)
            .await?;
        assert!(
            sqlx::query(statement).execute(&mut *tamper).await.is_err(),
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
    let foreign_dir = fence_parent().join(format!("oteryn-character-foreign-{}", database.name));
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
            .record()
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
    assert_eq!(transition.record().predecessor_digest, generation_one);
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
    let retained = fence_parent().join(format!("oteryn-character-fresh-{}", database.name));
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
    let retained = fence_parent().join(format!("oteryn-character-intent-{}", database.name));
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
    assign_world(&pool, &root, &node, 90, 95).await?;
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

#[test]
fn name_reservation_is_global_case_and_space_folded_and_fail_closed() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured");
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let database = Database::create(admin, "names").await?;
            let result = name_matrix(&database).await;
            database.cleanup().await?;
            result
        })
}

/// CHAR-NAME-1: one global namespace keyed by the ASCII lower-case name without
/// spaces; a taken key refuses the bootstrap with zero authoritative writes.
async fn name_matrix(database: &Database) -> TestResult {
    let root = DurabilityRoot::connect_test_runtime(&database.url)?;
    assert!(root.maintain_ready_once().await?);
    let pool = sqlx::PgPool::connect(&database.url).await?;
    let retained = fence_parent().join(format!("oteryn-character-names-{}", database.name));
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
    assign_world(&pool, &root, &node, 90, 95).await?;
    assign_world(&pool, &root, &node, 91, 96).await?;
    configure(&pool, ["profile-1", "ruleset-1", "content-1", "starter-1"]).await?;
    let named = |operation: u8, account: u8, revision: i64, name: &str| -> TestResult<_> {
        let mut wire = Wire::new(operation, account, revision)?;
        wire.name = name.to_owned();
        wire.decode()
    };
    let unavailable = |result: Result<_, CharacterAuthorityError>| {
        matches!(result, Err(CharacterAuthorityError::NameUnavailable))
    };

    // The committed root carries the name and its reservation.
    allow(&root, &node, 81).await?;
    let first_intent = named(80, 81, 1, "Al Dric")?;
    let first = root
        .bootstrap_character(&authority, &node, &first_intent)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let stored: (String, String, String) = sqlx::query_as(
        "SELECT r.name, r.name_key, n.character_id::text FROM game_character_roots r \
           JOIN game_character_name_reservations n USING (name_key)",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        stored,
        (
            "Al Dric".to_owned(),
            "aldric".to_owned(),
            uuid(*first.character_id.as_bytes())
        )
    );
    // An exact retry reconciles its own reservation, never a conflict.
    assert_eq!(
        root.bootstrap_character(&authority, &node, &first_intent)
            .await
            .map_err(|e| format!("{e:?}"))?,
        first
    );

    // Case and spaces fold, in the same World and in another one.
    allow(&root, &node, 83).await?;
    let before = totals(&pool).await?;
    assert!(unavailable(
        root.bootstrap_character(&authority, &node, &named(82, 83, 2, "ALDRIC")?)
            .await
    ));
    let mut elsewhere = Wire::new(84, 83, 2)?;
    elsewhere.name = "aldric".to_owned();
    elsewhere.world = 91;
    assert!(unavailable(
        root.bootstrap_character(&authority, &node, &elsewhere.decode()?)
            .await
    ));
    assert_eq!(totals(&pool).await?, before);
    assert_eq!(floor(&pool).await?, 1);
    // A different key commits.
    root.bootstrap_character(&authority, &node, &named(85, 83, 2, "Aldrik")?)
        .await
        .map_err(|e| format!("{e:?}"))?;

    // Same-name race: exactly one winner, the other sees the reservation.
    allow(&root, &node, 87).await?;
    allow(&root, &node, 88).await?;
    let racing = [
        named(86, 87, 3, "Race Winner")?,
        named(89, 88, 4, "race winner")?,
    ];
    let outcomes = concurrently(&database.url, &fence, &node, &racing);
    assert_eq!(
        outcomes.iter().filter(|o| o.is_ok()).count(),
        1,
        "{outcomes:?}"
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| matches!(o, Err(e) if e == "NameUnavailable"))
            .count(),
        1,
        "{outcomes:?}"
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_character_name_reservations"
        )
        .await?,
        3
    );

    // The database enforces the namespace and immutability for every writer.
    let guarded = [
        "UPDATE game_character_roots SET name = 'Other', character_revision = character_revision + 1",
        "DELETE FROM game_character_name_reservations",
        "UPDATE game_character_name_reservations SET name_key = 'other'",
        "TRUNCATE game_character_name_reservations",
        "INSERT INTO game_character_account_guards VALUES ('01890f4c-3b2a-7cc2-8d11-9a321b7c0001'); \
         INSERT INTO game_character_roots VALUES ('01890f4c-3b2a-7cc2-8d11-9a321b7c0002','01890f4c-3b2a-7cc2-8d11-9a321b7c0001','01890f4c-3b2a-7cc2-8d11-9a321b7c0003',1,1,'p','r','c','s','Al dric')",
        "INSERT INTO game_character_account_guards VALUES ('01890f4c-3b2a-7cc2-8d11-9a321b7c0001'); \
         INSERT INTO game_character_roots VALUES ('01890f4c-3b2a-7cc2-8d11-9a321b7c0002','01890f4c-3b2a-7cc2-8d11-9a321b7c0001','01890f4c-3b2a-7cc2-8d11-9a321b7c0003',1,1,'p','r','c','s','Hero9')",
    ];
    for statement in guarded {
        let mut tx = pool.begin().await?;
        assert!(
            tx.execute(sqlx::AssertSqlSafe(statement)).await.is_err(),
            "{statement}"
        );
        tx.rollback().await?;
    }
    // The runtime role has no direct write on the reservations.
    let runtime_insert: bool = sqlx::query_scalar(
        "SELECT has_table_privilege('oteryn_game_runtime', 'game_character_name_reservations', 'INSERT')",
    )
    .fetch_one(&pool)
    .await?;
    assert!(!runtime_insert);

    // A restored store whose root lost its reservation keeps authority closed.
    drop(authority);
    let mut tamper = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tamper)
        .await?;
    let kept: (String, i64) = sqlx::query_as(
        "SELECT name_key, reserved_at FROM game_character_name_reservations WHERE name_key = 'aldric'",
    )
    .fetch_one(&mut *tamper)
    .await?;
    sqlx::query("DELETE FROM game_character_name_reservations WHERE name_key = 'aldric'")
        .execute(&mut *tamper)
        .await?;
    tamper.commit().await?;
    assert!(root.open_character_authority(&fence).await.is_err());
    let mut repair = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *repair)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_name_reservations VALUES ($1, 1, encode($2,'hex')::uuid, $3)",
    )
    .bind(&kept.0)
    .bind(first.character_id.as_bytes().as_slice())
    .bind(kept.1)
    .execute(&mut *repair)
    .await?;
    repair.commit().await?;
    root.open_character_authority(&fence)
        .await
        .map_err(|e| format!("{e:?}"))?;

    drop(fence);
    pool.close().await;
    std::fs::remove_dir_all(retained)?;
    Ok(())
}

#[test]
fn name_migration_refuses_a_store_that_already_holds_a_character() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured");
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let database = Database::create_at(admin, "name_preproduction", Some(21)).await?;
            let result = name_migration_refusal(&database).await;
            database.cleanup().await?;
            result
        })
}

/// CHAR-NAME-1 preproduction-only assumption (§6.1): 0022 adds a NOT NULL name
/// without a default, so on a store that already holds a Character root it
/// refuses to apply and leaves the root and the migration history unchanged.
async fn name_migration_refusal(database: &Database) -> TestResult {
    let mut connection = sqlx::PgConnection::connect(&database.url).await?;
    sqlx::query("INSERT INTO game_character_account_guards VALUES (encode($1,'hex')::uuid)")
        .bind(id(40).as_slice())
        .execute(&mut connection)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_roots VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          1,1,'profile-1','ruleset-1','content-1','starter-1')",
    )
    .bind(id(41).as_slice())
    .bind(id(40).as_slice())
    .bind(id(42).as_slice())
    .execute(&mut connection)
    .await?;

    let refused = sqlx::migrate!("./migrations").run(&mut connection).await;
    let Err(sqlx::migrate::MigrateError::ExecuteMigration(error, 22)) = refused else {
        return Err(format!("0022 must refuse a store with a Character: {refused:?}").into());
    };
    assert_eq!(
        error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("23502")
    );

    let roots: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_roots")
        .fetch_one(&mut connection)
        .await?;
    assert_eq!(roots, 1);
    let latest: i64 = sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations")
        .fetch_one(&mut connection)
        .await?;
    assert_eq!(latest, 21);
    let named: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.columns \
         WHERE table_name = 'game_character_roots' AND column_name = 'name')",
    )
    .fetch_one(&mut connection)
    .await?;
    assert!(!named);
    connection.close().await?;
    Ok(())
}

#[test]
fn fresh_store_rerun_is_idempotent_and_interpretation_waits_for_it() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured");
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let database = Database::create(admin, "fresh_rerun").await?;
            let result = fresh_rerun(&database).await;
            database.cleanup().await?;
            result
        })
}

/// OPS-NODE-BOOT-01 D2: the interpretation is refused until generation one is
/// admitted, and a re-run with the retained request after a lost
/// acknowledgement returns the admitted record without a second fresh store.
async fn fresh_rerun(database: &Database) -> TestResult {
    let root = DurabilityRoot::connect_test_runtime(&database.url)?;
    assert!(root.maintain_ready_once().await?);
    assert!(matches!(
        root.configure_character_interpretation("p", "r", "c", "s")
            .await,
        Err(CharacterAuthorityError::Rejected)
    ));
    let retained = fence_parent().join(format!("oteryn-character-rerun-{}", database.name));
    let _ = std::fs::remove_dir_all(&retained);
    std::fs::create_dir(&retained)?;
    let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
        .map_err(|e| format!("{e:?}"))?;
    for _ in 0..2 {
        let fresh = recovery
            .authorize_fresh_store(id(11), 100)
            .map_err(|e| format!("{e:?}"))?;
        root.admit_fresh_character_recovery(&fresh)
            .await
            .map_err(|e| format!("{e:?}"))?;
    }
    assert_eq!(
        root.configure_character_interpretation("p", "r", "c", "s")
            .await
            .map_err(|e| format!("{e:?}"))?,
        1
    );
    // A re-run after other Character state exists still returns the record.
    let fresh = recovery
        .authorize_fresh_store(id(11), 100)
        .map_err(|e| format!("{e:?}"))?;
    root.admit_fresh_character_recovery(&fresh)
        .await
        .map_err(|e| format!("{e:?}"))?;
    drop(fresh);
    // Different inputs never authorize a second fresh store.
    assert!(recovery.authorize_fresh_store(id(12), 100).is_err());
    let pool = sqlx::PgPool::connect(&database.url).await?;
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_character_recovery_admissions"
        )
        .await?,
        1
    );
    pool.close().await;
    std::fs::remove_dir_all(retained)?;
    Ok(())
}

// R7 P03 shares its cases with the focused standalone target so the established
// protected PostgreSQL 17.6 lane executes the exact same qualification.
#[path = "support/character_progression_postgres_cases.rs"]
mod character_progression_postgres_cases;

// DUR-03 stage C one-item Ground MINT shares its cases with the focused
// standalone target through the same protected PostgreSQL lane.
#[path = "support/item_mint_postgres_cases.rs"]
mod item_mint_postgres_cases;

// B3-1 TRANSFER (Ground -> container slot / main backpack entries) shares its
// cases with the focused standalone target through the same protected lane.
#[path = "support/item_transfer_postgres_cases.rs"]
mod item_transfer_postgres_cases;

// D3-4 corpse-container TRANSFER (D133 window, corpse never a source) shares
// its cases with the focused standalone target through the same protected lane.
#[path = "support/corpse_transfer_postgres_cases.rs"]
mod corpse_transfer_postgres_cases;

// D3-6 corpse DECAY_RETIRE (N+1 one-item steps at materialized_at + 60 s,
// resumable from durable state) shares its cases with the focused standalone
// target through the same protected lane.
#[path = "support/corpse_decay_postgres_cases.rs"]
mod corpse_decay_postgres_cases;

// DEATH-0 death receipts, blessings and pending respawns (migration 0016)
// share their cases with the focused standalone target through the same
// protected lane.
#[path = "support/character_death_receipts_postgres_cases.rs"]
mod character_death_receipts_postgres_cases;

// PRIV-GUARD-1 (the runtime role executes every function a CHECK calls) shares
// its cases with the focused standalone target through the same protected lane.
#[path = "support/check_function_privileges_postgres_cases.rs"]
mod check_function_privileges_postgres_cases;

// HOUSE-CUSTODY-1 HouseInterior, reclaim provenance and the item location
// exclusivity guard (migration 0025) share their cases with the focused
// standalone target through the same protected lane.
#[path = "support/house_custody_postgres_cases.rs"]
mod house_custody_postgres_cases;
// INBOX-1a CharacterInbox and delivery (migration 0076) share their cases
// with the focused standalone target through the same protected lane.
#[path = "support/character_inbox_postgres_cases.rs"]
mod character_inbox_postgres_cases;

// STANCE-0 stance slot and stance receipts (migration 0017) share their cases
// with the focused standalone target through the same protected lane.
#[path = "support/character_stance_postgres_cases.rs"]
mod character_stance_postgres_cases;

// CHARM-3 charm unlocks, assignments and receipts (migration 0020) share their
// cases with the focused standalone target through the same protected lane.
#[path = "support/charm_state_postgres_cases.rs"]
mod charm_state_postgres_cases;
#[path = "../src/gameplay_transport/charm.rs"]
mod charm_transport;

// CHEST-1 reward-claim MINT (a `once` RewardClaim into a new main backpack
// entry) shares its cases with the focused standalone target through the same
// protected lane.
#[path = "support/reward_claim_mint_postgres_cases.rs"]
mod reward_claim_mint_postgres_cases;

// CHARM-2 Bestiary kill progress (migration 0019) shares its cases and their
// harness with the focused standalone target through the same protected lane.
#[allow(dead_code)]
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
#[path = "support/bestiary_progress_postgres_cases.rs"]
mod bestiary_progress_postgres_cases;

// ACHIEVEMENT step 3 account facts (migration 0021) share their cases with the
// focused standalone target through the same protected lane, on the CHARM-2
// harness included above.
#[path = "support/account_achievement_postgres_cases.rs"]
mod account_achievement_postgres_cases;

// LCFA-1 `ListCharactersForAccount` outbox, revision and epoch (migration 0024)
// share their cases with the protected PostgreSQL lane.
#[path = "support/account_characters_projection_postgres_cases.rs"]
mod account_characters_projection_postgres_cases;

// PREM-1a Premium consumer fence (migration 0029) runs in the same protected lane, on the
// CHARM-2 harness included above.
#[path = "support/premium_fence_postgres_cases.rs"]
mod premium_fence_postgres_cases;

// CHAR-BUILD-1a build state, build receipts and death build fields (migration
// 0030) and their admission verifier checks, on the CHARM-2 harness included above.
#[path = "support/character_build_postgres_cases.rs"]
mod character_build_postgres_cases;

#[path = "support/character_stance_writer_postgres_cases.rs"]
mod character_stance_writer_postgres_cases;

#[path = "support/character_familiar_writer_postgres_cases.rs"]
mod character_familiar_writer_postgres_cases;

// QUEST-STATE-1 quest tracks, states and receipts (migration 0056) and their writer, on the
// CHARM-2 harness included above.
#[path = "support/quest_state_postgres_cases.rs"]
mod quest_state_postgres_cases;

// WHEEL-W1 Wheel allocation, receipts, writer and admission reset (migration 0070), on the
// CHARM-2 harness included above.
#[path = "support/character_wheel_postgres_cases.rs"]
mod character_wheel_postgres_cases;

// SPELL-D8 H-1 durable monk Harmony and remaining forced Serene time (migration
// 0026, `durability::monk_state`) run in the same protected lane, on the
// CHARM-2 harness included above.
mod monk_state_postgres_cases {
    use super::{Database, id};
    use crate::bestiary_postgres_harness::{
        CHARACTER, Harness, TestResult, configured_admin, context, debug, fence, runtime,
    };
    use crate::domain::CharacterId;
    use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
    use crate::durability::character_death::{
        CharacterDeathOutcome, CharacterDeathRequest, DeathCell, PlayerDeathOccurrence,
    };
    use crate::durability::character_progression::CharacterProgressionError;
    use crate::durability::monk_state::{
        DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveOutcome, MonkStateSaveRequest,
    };
    use crate::durability::{DurabilityError, DurabilityRoot};
    use crate::foundation::{ChannelId, ConnectionGeneration, WorldId};
    use oteryn_simulation_determinism::{ExactI64, RoundingMode};
    use sqlx::Connection;

    fn state(harmony: u8, micros: u64) -> TestResult<DurableMonkState> {
        Ok(DurableMonkState::new(harmony, micros).map_err(debug)?)
    }

    fn save(tag: u8, harmony: u8, micros: u64) -> TestResult<MonkStateSaveRequest> {
        Ok(MonkStateSaveRequest {
            occurrence: MonkStateSaveOccurrence::from_bytes(id(tag)).map_err(debug)?,
            state: state(harmony, micros)?,
        })
    }

    /// Global thresholds of levels 1-10, as the DEATH-1 writer cases.
    const GLOBAL: [i64; 11] = [0, 100, 200, 400, 800, 1500, 2600, 4200, 6400, 9300, 13000];

    fn death(tag: u8) -> TestResult<CharacterDeathRequest<10>> {
        let mut thresholds = [LevelThreshold {
            level: 1,
            minimum_experience: ExactI64::new(0),
        }; 10];
        for (index, threshold) in thresholds.iter_mut().enumerate() {
            *threshold = LevelThreshold {
                level: u32::try_from(index + 1)?,
                minimum_experience: ExactI64::new(GLOBAL[index]),
            };
        }
        Ok(CharacterDeathRequest {
            occurrence: PlayerDeathOccurrence::from_bytes(id(tag)).map_err(debug)?,
            context: context(),
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            policy: FiniteProgressionPolicy {
                context: context(),
                policy_revision: "policy-1".into(),
                reward_revision: "reward-1".into(),
                death_policy_revision: "death-1".into(),
                declared_difference_revision: "declaration-1".into(),
                thresholds,
                terminal_exclusive_experience: ExactI64::new(GLOBAL[10]),
                death_loss_numerator: 1,
                death_loss_denominator: 1,
                death_loss_rounding: RoundingMode::Floor,
            },
            held_blessings: Vec::new(),
            death_cell: DeathCell {
                world_id: WorldId::decode(&id(42)).map_err(debug)?,
                channel_id: ChannelId::decode(&id(43)).map_err(debug)?,
                spatial_position: vec![1, 2, 3, 7],
                map_revision: "map-1".into(),
            },
            respawn_position: b"temple:thais".to_vec(),
        })
    }

    /// Harmony, forced time, CharacterRevision and monk receipt count as stored.
    async fn stored(harness: &Harness) -> TestResult<(i16, i64, String, i64)> {
        let (harmony, micros): (i16, i64) = sqlx::query_as(
            "SELECT harmony, serene_forced_remaining_micros \
               FROM game_character_progression_state",
        )
        .fetch_one(&harness.pool)
        .await?;
        Ok((
            harmony,
            micros,
            harness.root_revision().await?,
            harness.count("game_character_monk_state_receipts").await?,
        ))
    }

    fn database_code(error: &sqlx::Error) -> Option<String> {
        error
            .as_database_error()
            .and_then(|error| error.code())
            .map(|code| code.into_owned())
    }

    /// The guard rule that rejected: the message of its RAISE.
    fn database_message(error: &sqlx::Error) -> String {
        error
            .as_database_error()
            .map(|error| error.message().to_owned())
            .unwrap_or_default()
    }

    #[test]
    fn migration_gives_existing_rows_zero_and_storage_rejects_out_of_range() -> TestResult {
        let Some(admin) = configured_admin() else {
            return Ok(());
        };
        runtime()?.block_on(async move {
            let database = Database::create_at(admin, "monk_default", Some(24)).await?;
            let result = existing_rows(&database).await;
            database.cleanup().await?;
            result
        })
    }

    async fn existing_rows(database: &Database) -> TestResult {
        let mut connection = sqlx::PgConnection::connect(&database.url).await?;
        for statement in [
            "INSERT INTO game_character_interpretations VALUES \
             (1,'profile-1','ruleset-1','content-1','starter-1',1)",
            "INSERT INTO game_character_account_guards VALUES (encode($1,'hex')::uuid)",
        ] {
            let query = sqlx::query(statement);
            let query = if statement.contains("$1") {
                query.bind(id(40).as_slice())
            } else {
                query
            };
            query.execute(&mut connection).await?;
        }
        sqlx::query(
            "INSERT INTO game_character_roots VALUES \
             (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
              1,1,'profile-1','ruleset-1','content-1','starter-1','Fixture Hero')",
        )
        .bind(id(CHARACTER).as_slice())
        .bind(id(40).as_slice())
        .bind(id(42).as_slice())
        .execute(&mut connection)
        .await?;
        sqlx::query(
            "INSERT INTO game_character_progression_state VALUES \
             (encode($1,'hex')::uuid,1,50,1000,'profile-1','ruleset-1','content-1',\
              'simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
        )
        .bind(id(CHARACTER).as_slice())
        .execute(&mut connection)
        .await?;

        sqlx::migrate!("./migrations").run(&mut connection).await?;
        let values: (i16, i64) = sqlx::query_as(
            "SELECT harmony, serene_forced_remaining_micros \
               FROM game_character_progression_state",
        )
        .fetch_one(&mut connection)
        .await?;
        assert_eq!(
            values,
            (0, 0),
            "an existing row reads Harmony 0 and no forced time"
        );

        // The CHECKs alone (triggers disabled): storage rejects every other value.
        for (column, value, constraint) in [
            (
                "harmony",
                6_i64,
                "game_character_progression_state_harmony_range",
            ),
            (
                "harmony",
                -1,
                "game_character_progression_state_harmony_range",
            ),
            (
                "serene_forced_remaining_micros",
                7_000_001,
                "game_character_progression_state_serene_forced_range",
            ),
            (
                "serene_forced_remaining_micros",
                -1,
                "game_character_progression_state_serene_forced_range",
            ),
        ] {
            let mut tx = connection.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            let rejected = sqlx::query(sqlx::AssertSqlSafe(format!(
                "UPDATE game_character_progression_state SET {column} = {value}"
            )))
            .execute(&mut *tx)
            .await
            .err()
            .ok_or_else(|| format!("{column} = {value} must be rejected"))?;
            assert_eq!(database_code(&rejected).as_deref(), Some("23514"));
            assert_eq!(
                rejected
                    .as_database_error()
                    .and_then(|error| error.constraint()),
                Some(constraint)
            );
            tx.rollback().await?;
        }
        for (harmony, micros) in [(5, 7_000_000), (0, 0)] {
            let mut tx = connection.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE game_character_progression_state \
                    SET harmony = $1, serene_forced_remaining_micros = $2",
            )
            .bind(harmony)
            .bind(micros)
            .execute(&mut *tx)
            .await?;
            tx.rollback().await?;
        }
        connection.close().await?;
        Ok(())
    }

    #[test]
    fn actor_end_save_is_fenced_replayed_and_emptied_by_death() -> TestResult {
        let Some(admin) = configured_admin() else {
            return Ok(());
        };
        runtime()?.block_on(async move {
            let harness = Harness::create(admin, "monk_save", false).await?;
            let result = save_flow(&harness).await;
            harness.cleanup().await?;
            result
        })
    }

    async fn save_flow(harness: &Harness) -> TestResult {
        // A level-9 Character at 6500 experience (the DEATH-1 writer fixture).
        sqlx::query(
            "INSERT INTO game_character_progression_state VALUES \
             (encode($1,'hex')::uuid,1,9,6500,'profile-1','ruleset-1','content-1',\
              'simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
        )
        .bind(id(CHARACTER).as_slice())
        .execute(&harness.pool)
        .await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let root: &DurabilityRoot = &harness.root;
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;

        assert_eq!(
            root.read_character_monk_state(&authority, character)
                .await
                .map_err(debug)?,
            DurableMonkState::default()
        );
        assert!(MonkStateSaveOccurrence::from_bytes([7; 16]).is_err());

        // Actor end with Harmony 3 and 2.5 s of forced Serene left.
        let first = root
            .commit_character_monk_state_save(
                &authority,
                &harness.node,
                fence(1)?,
                save(70, 3, 2_500_000)?,
            )
            .await
            .map_err(debug)?;
        let MonkStateSaveOutcome::Committed(committed) = first else {
            return Err(format!("unexpected first save: {first:?}").into());
        };
        assert_eq!(committed.before, DurableMonkState::default());
        assert_eq!(committed.after, state(3, 2_500_000)?);
        assert_eq!(committed.original_character_revision.get(), 1);
        assert_eq!(committed.committed_character_revision.get(), 2);
        assert_eq!(stored(harness).await?, (3, 2_500_000, "2".into(), 1));
        assert_eq!(
            root.read_character_monk_state(&authority, character)
                .await
                .map_err(debug)?,
            state(3, 2_500_000)?
        );

        // A restart between the save and the load: a new DurabilityRoot with its own pool over
        // the same database, and the retained recovery seal, reads the stored values back.
        {
            let restarted = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
            assert!(restarted.maintain_ready_once().await?);
            let reopened = restarted
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            assert_eq!(
                restarted
                    .read_character_monk_state(&reopened, character)
                    .await
                    .map_err(debug)?,
                state(3, 2_500_000)?
            );
        }

        // Exact replay, even at the now stale revision, returns the receipt; changed reuse
        // conflicts; reconciliation proves what committed.
        let replay = root
            .commit_character_monk_state_save(
                &authority,
                &harness.node,
                fence(1)?,
                save(70, 3, 2_500_000)?,
            )
            .await
            .map_err(debug)?;
        assert_eq!(replay, MonkStateSaveOutcome::AlreadyCommitted(committed));
        let conflict = root
            .commit_character_monk_state_save(&authority, &harness.node, fence(1)?, save(70, 4, 0)?)
            .await;
        assert!(
            matches!(
                conflict,
                Err(CharacterProgressionError::ConflictingOccurrence)
            ),
            "{conflict:?}"
        );
        assert_eq!(
            root.reconcile_character_monk_state_save(
                &authority,
                MonkStateSaveOccurrence::from_bytes(id(70)).map_err(debug)?
            )
            .await
            .map_err(debug)?,
            Some(committed)
        );
        assert_eq!(
            root.reconcile_character_monk_state_save(
                &authority,
                MonkStateSaveOccurrence::from_bytes(id(79)).map_err(debug)?
            )
            .await
            .map_err(debug)?,
            None
        );

        // An unchanged save proves the fence and writes nothing.
        let unchanged = root
            .commit_character_monk_state_save(
                &authority,
                &harness.node,
                fence(2)?,
                save(71, 3, 2_500_000)?,
            )
            .await
            .map_err(debug)?;
        assert_eq!(unchanged, MonkStateSaveOutcome::Unchanged);
        assert_eq!(stored(harness).await?, (3, 2_500_000, "2".into(), 1));

        // A stale fence writes nothing: each case changes exactly one fact.
        let mut other_connection = fence(2)?;
        other_connection.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        let mut other_lease = fence(2)?;
        other_lease.character_lease_generation = 2;
        let mut other_session = fence(2)?;
        other_session.game_session_id =
            crate::foundation::GameSessionId::decode(&id(51)).map_err(debug)?;
        for (tag, stale, case) in [
            (72, other_connection, "another connection generation"),
            (73, other_lease, "another lease generation"),
            (74, other_session, "another game session"),
        ] {
            let outcome = root
                .commit_character_monk_state_save(
                    &authority,
                    &harness.node,
                    stale,
                    save(tag, 5, 0)?,
                )
                .await;
            assert!(
                matches!(outcome, Err(CharacterProgressionError::AuthorityRejected)),
                "{case}: {outcome:?}"
            );
            assert_eq!(
                stored(harness).await?,
                (3, 2_500_000, "2".into(), 1),
                "{case}"
            );
        }
        let stale_revision = root
            .commit_character_monk_state_save(&authority, &harness.node, fence(1)?, save(75, 5, 0)?)
            .await;
        assert!(
            matches!(
                stale_revision,
                Err(CharacterProgressionError::CharacterRevisionMismatch)
            ),
            "{stale_revision:?}"
        );
        // The session moved to a newer connection generation: the actor's own (older)
        // generation is stale and its end write is fenced out.
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions SET current_generation = 2 \
              WHERE game_session_id = encode($1,'hex')::uuid",
        )
        .bind(id(50).as_slice())
        .execute(&harness.pool)
        .await?;
        let superseded = root
            .commit_character_monk_state_save(&authority, &harness.node, fence(2)?, save(76, 5, 0)?)
            .await;
        assert!(
            matches!(
                superseded,
                Err(CharacterProgressionError::AuthorityRejected)
            ),
            "{superseded:?}"
        );
        assert_eq!(stored(harness).await?, (3, 2_500_000, "2".into(), 1));
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions SET current_generation = 1 \
              WHERE game_session_id = encode($1,'hex')::uuid",
        )
        .bind(id(50).as_slice())
        .execute(&harness.pool)
        .await?;

        // The 0026 guard: a death transition that leaves Harmony non-zero is rejected by the death
        // rule. The death's own receipt, pending respawn and CharacterRevision step are exact, so
        // only Harmony is wrong.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("UPDATE game_character_roots SET character_revision = 3")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_character_progression_state SET character_revision = 3")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO game_character_death_receipts(\
               death_occurrence_id, command_binding, policy_digest, character_id, \
               original_character_revision, committed_character_revision, level_before, \
               level_after, experience_before, experience_after, experience_lost, \
               blessings_before, blessings_after, amulet_of_loss_item_id, lost_item_ids, \
               death_world_id, death_channel_id, death_spatial_position, death_map_revision, \
               respawn_position, death_policy_revision, profile_revision, ruleset_revision, \
               content_revision, simulation_revision, evidence_revision, declaration_revision, \
               policy_revision, reward_revision, committed_at) \
             SELECT encode($1,'hex')::uuid, $2, $3, r.character_id, 2, 3, 9, 9, 6500, 6500, 0, \
               '{}', '{}', NULL, '{}', r.world_id, encode($4,'hex')::uuid, '\\x01'::bytea, \
               'map-1', 'temple:thais'::bytea, 'death-1', 'profile-1', 'ruleset-1', \
               'content-1', 'simulation-1', 'evidence-1', 'declaration-1', 'policy-1', \
               'reward-1', 3 \
               FROM game_character_roots r WHERE r.character_id = encode($5,'hex')::uuid",
        )
        .bind(id(83).as_slice())
        .bind([83_u8; 33].as_slice())
        .bind([83_u8; 32].as_slice())
        .bind(id(43).as_slice())
        .bind(id(CHARACTER).as_slice())
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO game_character_pending_respawns VALUES \
             (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 'temple:thais'::bytea)",
        )
        .bind(id(CHARACTER).as_slice())
        .bind(id(83).as_slice())
        .execute(&mut *tx)
        .await?;
        let kept_harmony = tx
            .commit()
            .await
            .err()
            .ok_or("a death transition that keeps Harmony must fail")?;
        assert_eq!(database_code(&kept_harmony).as_deref(), Some("23514"));
        assert_eq!(
            database_message(&kept_harmony),
            "a Character death must empty Harmony and the forced Serene time"
        );
        assert_eq!(stored(harness).await?, (3, 2_500_000, "2".into(), 1));
        assert_eq!(harness.count("game_character_death_receipts").await?, 0);

        // DEATH-1 empties both values in its own Character transaction.
        let died = root
            .commit_character_death(&authority, &harness.node, fence(2)?, death(80)?)
            .await
            .map_err(debug)?;
        assert!(
            matches!(died, CharacterDeathOutcome::Committed(_)),
            "{died:?}"
        );
        assert_eq!(stored(harness).await?, (0, 0, "3".into(), 1));
        assert_eq!(
            root.read_character_monk_state(&authority, character)
                .await
                .map_err(debug)?,
            DurableMonkState::default()
        );
        // While the respawn is pending no save changes the death's zeros.
        let pending = root
            .commit_character_monk_state_save(&authority, &harness.node, fence(3)?, save(77, 1, 0)?)
            .await;
        assert!(
            matches!(pending, Err(CharacterProgressionError::RespawnPending)),
            "{pending:?}"
        );
        assert_eq!(
            root.commit_character_monk_state_save(
                &authority,
                &harness.node,
                fence(3)?,
                save(78, 0, 0)?
            )
            .await
            .map_err(debug)?,
            MonkStateSaveOutcome::Unchanged
        );
        assert_eq!(stored(harness).await?, (0, 0, "3".into(), 1));

        // The 0026 guard: a transition without a monk state receipt cannot change Harmony.
        let (level, experience): (i64, i64) =
            sqlx::query_as("SELECT level, total_experience FROM game_character_progression_state")
                .fetch_one(&harness.pool)
                .await?;
        let mut tx = harness.pool.begin().await?;
        sqlx::query("UPDATE game_character_roots SET character_revision = 4")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE game_character_progression_state SET character_revision = 4, harmony = 2",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO game_character_stance_receipts(\
               stance_occurrence_id, command_binding, policy_digest, character_id, \
               original_character_revision, committed_character_revision, level_before, \
               level_after, experience_before, experience_after, stance_before, stance_after, \
               profile_revision, ruleset_revision, content_revision, simulation_revision, \
               evidence_revision, declaration_revision, policy_revision, reward_revision, \
               committed_at) \
             VALUES (encode($1,'hex')::uuid, $2, $3, encode($4,'hex')::uuid, 3, 4, $5, $5, \
               $6, $6, NULL, 'stance-a', 'profile-1', 'ruleset-1', 'content-1', \
               'simulation-1', 'evidence-1', 'declaration-1', 'policy-1', 'reward-1', 3)",
        )
        .bind(id(81).as_slice())
        .bind([81_u8; 33].as_slice())
        .bind([81_u8; 32].as_slice())
        .bind(id(CHARACTER).as_slice())
        .bind(level)
        .bind(experience)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO game_character_stance VALUES \
             (encode($1,'hex')::uuid, 'stance-a', 4, encode($2,'hex')::uuid)",
        )
        .bind(id(CHARACTER).as_slice())
        .bind(id(81).as_slice())
        .execute(&mut *tx)
        .await?;
        let guarded = tx
            .commit()
            .await
            .err()
            .ok_or("a Harmony change without a monk state receipt must fail")?;
        assert_eq!(database_code(&guarded).as_deref(), Some("23514"));
        assert_eq!(
            database_message(&guarded),
            "Harmony and the forced Serene time change only with a monk state receipt"
        );
        assert_eq!(stored(harness).await?, (0, 0, "3".into(), 1));

        // A stored value outside the bounds fails the load closed (the CHECK is dropped in this
        // throwaway database to plant it).
        for (constraint, column, value) in [
            (
                "game_character_progression_state_harmony_range",
                "harmony",
                6_i64,
            ),
            (
                "game_character_progression_state_serene_forced_range",
                "serene_forced_remaining_micros",
                7_000_001,
            ),
        ] {
            let mut tx = harness.pool.begin().await?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "ALTER TABLE game_character_progression_state DROP CONSTRAINT {constraint}"
            )))
            .execute(&mut *tx)
            .await?;
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE game_character_progression_state \
                    SET harmony = 0, serene_forced_remaining_micros = 0",
            )
            .execute(&mut *tx)
            .await?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "UPDATE game_character_progression_state SET {column} = {value}"
            )))
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            let corrupt = root.read_character_monk_state(&authority, character).await;
            assert!(
                matches!(
                    corrupt,
                    Err(CharacterProgressionError::Unavailable(
                        DurabilityError::InvalidStoredState
                    ))
                ),
                "{column}: {corrupt:?}"
            );
            let blocked = root
                .commit_character_monk_state_save(
                    &authority,
                    &harness.node,
                    fence(3)?,
                    save(82, 0, 0)?,
                )
                .await;
            assert!(
                matches!(
                    blocked,
                    Err(CharacterProgressionError::Unavailable(
                        DurabilityError::InvalidStoredState
                    ))
                ),
                "{column}: {blocked:?}"
            );
        }
        drop(authority);
        drop(seal);
        Ok(())
    }
}
// Combat D2b creature death -> loot MINT + R7 P03 XP composition shares its
// cases with the focused standalone target through the same protected lane.
#[allow(dead_code, unused_imports)]
#[path = "../src/combat.rs"]
pub mod combat;
#[path = "support/combat_death_reward_postgres_cases.rs"]
mod combat_death_reward_postgres_cases;

// CHARM-2 Bestiary descendant of a committed creature death shares its cases
// with the focused standalone target, on the CHARM-2 harness included above.
#[path = "support/combat_bestiary_postgres_cases.rs"]
mod combat_bestiary_postgres_cases;

// B3-2 Combat ground pickup (definition facts bound to the current Content
// generation) shares its cases with the focused standalone target, on the B3-1
// and D3-4 cases included above. `combat_pickup` is a top-level module and
// `content` is the test shim for the same reasons as in that target.
#[allow(dead_code, unused_imports)]
#[path = "../src/combat/pickup.rs"]
pub mod combat_pickup;
#[path = "support/combat_pickup_postgres_cases.rs"]
mod combat_pickup_postgres_cases;
#[allow(dead_code, unused_imports)]
#[path = "support/content_shim.rs"]
pub mod content;

// D39 chest `USE` wiring to the CHEST-1 reward-claim MINT shares its cases with
// the focused standalone target through the same protected lane.
#[allow(dead_code, unused_imports)]
#[path = "../src/achievement_catalogue.rs"]
pub mod achievement_catalogue;
#[path = "support/chest_use_postgres_cases.rs"]
mod chest_use_postgres_cases;
#[allow(dead_code, unused_imports)]
#[path = "../src/interaction/mod.rs"]
pub mod interaction;
#[allow(dead_code, unused_imports)]
#[path = "../src/interaction/chest_use.rs"]
pub mod interaction_chest_use;

// GOLD-FEE-1a in-transaction gold fee BURN (migration 0023) shares its cases
// with the focused standalone target through the same protected lane.
#[path = "support/item_fee_burn_postgres_cases.rs"]
mod item_fee_burn_postgres_cases;

// BANK-1 Account bank balance (migration 0071) shares its cases with the focused
// standalone target through the same protected lane, on the Bestiary harness.
#[path = "support/bank_postgres_cases.rs"]
mod bank_postgres_cases;
#[path = "support/gold_fee_bank_postgres_cases.rs"]
mod gold_fee_bank_postgres_cases;

// PG-COVERAGE-1: fails when a standalone `*_postgres.rs` target has cases that
// no CI-run PostgreSQL target includes. Runs without a database.
#[path = "support/postgres_target_aggregation.rs"]
mod postgres_target_aggregation;

// Both new inert public consumers resolve current recovery evidence independently of history.
#[test]
fn proficiency_read_public_consumers_reject_current_recovery_provenance_substitution()
-> bestiary_postgres_harness::TestResult {
    use bestiary_postgres_harness::{CHARACTER, Harness, configured_admin, debug, runtime};
    use durability::DurabilityError;
    use durability::character_progression::CharacterProgressionError::Unavailable;
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let harness = Harness::create(admin, "proficiency_inert_read_fence", true).await?;
        {
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut restore = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *restore)
                .await?;
            // Only current DB issuance changes; the independent sealed record stays untouched.
            sqlx::query("UPDATE game_character_recovery_admissions SET issued_at=issued_at+1")
                .execute(&mut *restore)
                .await?;
            restore.commit().await?;
            let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
            let root = &harness.root;
            for outcome in [
                root.read_character_proficiency_state(&authority, character)
                    .await
                    .map(|_| ()),
                root.read_character_proficiency_occurrence(&authority, character, id(90))
                    .await
                    .map(|_| ()),
            ] {
                // Require the actual fence disposition, not absent migration/schema 42P01.
                assert!(
                    matches!(outcome, Err(Unavailable(DurabilityError::Unavailable))),
                    "{outcome:?}"
                );
            }
        }
        harness.cleanup().await
    })
}

#[path = "support/character_proficiency_postgres_cases.rs"]
mod character_proficiency_postgres_cases;
