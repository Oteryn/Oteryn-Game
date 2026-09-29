// Include the production durability implementation in this dedicated PostgreSQL
// target. Ordinary workspace runs may skip when the routed PostgreSQL service is
// absent; only configured PostgreSQL 17.6 runs count as qualification evidence.
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

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use durability::native_admission_source::{
    DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
    PendingPublication, SourceObservation,
};
use durability::recovery_evidence_composition::RecoveryEvidenceSubject;
use durability::runtime_scope_assignment::{
    BootstrapSecret, LaunchBinding, NodeIncarnationProof, NodeRegistrationFact,
};
use durability::{DurabilityError, DurabilityRoot};
use ed25519_dalek::{Signer, SigningKey};
use foundation::NodeId;
use foundation::fnd04_verifier::RecoveryCurrentEvidence;
use foundation::{CharacterId, Fnd04ConsumerError, WorldId};
use sqlx::{Connection, Executor};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

const ACCOUNT_ID: &str = "01890f4c-3b2a-7cc2-8d11-9a321b7c0001";
const FRESH_ISSUER: &str = "urn:oteryn:platform:game-admission";
const FRESH_PROFILE: &str = "oteryn-pre-admission-v1";
const RECOVERY_ISSUER: &str = "urn:oteryn:platform:game-recovery";
const RECOVERY_PROFILE: &str = "oteryn-reauth-recovery-v1";

fn configured() -> bool {
    env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_some()
}

fn skipped() {
    eprintln!("PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured");
}

async fn create_database(
    test_name: &str,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    let admin_url = env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
    if !admin_url.starts_with("postgresql://oteryn_test_admin:")
        || !admin_url.ends_with("@127.0.0.1:5432/postgres")
    {
        return Err("unsafe PostgreSQL test admin URL".into());
    }
    let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let database_name = format!("{test_name}_{suffix}");
    let mut admin = sqlx::PgConnection::connect(&admin_url).await?;
    admin
        .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
            "CREATE DATABASE {database_name}"
        ))))
        .await?;
    admin.close().await?;
    let prefix = admin_url
        .strip_suffix("/postgres")
        .ok_or("invalid admin URL")?;
    let database_url = format!("{prefix}/{database_name}");
    Ok((admin_url, database_name, database_url))
}

async fn cleanup_database(
    admin_url: &str,
    database_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut admin = sqlx::PgConnection::connect(admin_url).await?;
    admin
        .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP DATABASE {database_name} WITH (FORCE)"
        ))))
        .await?;
    admin.close().await?;
    Ok(())
}

async fn migrate_postgres_17_6(database_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut connection = sqlx::PgConnection::connect(database_url).await?;
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&mut connection)
        .await?;
    assert_eq!(
        version, "170006",
        "canonical target requires PostgreSQL 17.6"
    );
    sqlx::migrate!("./migrations").run(&mut connection).await?;
    connection.close().await?;
    Ok(())
}

async fn ready_root(database_url: &str) -> Result<DurabilityRoot, DurabilityError> {
    let root = DurabilityRoot::connect_test_runtime(database_url)?;
    assert!(root.maintain_ready_once().await?);
    assert!(root.is_ready());
    Ok(root)
}

fn node_id(tag: u8) -> Result<NodeId, Box<dyn std::error::Error>> {
    Ok(NodeId::decode(&[
        0x01, 0x89, 0x0f, 0x4c, 0x3b, 0x2a, 0x7c, tag, 0x8d, 0x11, 0x9a, 0x32, 0x1b, 0x7c, 0x00,
        tag,
    ])
    .map_err(|error| format!("{error:?}"))?)
}

async fn register_node(
    root: &DurabilityRoot,
    tag: u8,
) -> Result<NodeIncarnationProof, Box<dyn std::error::Error>> {
    register_node_superseding(root, tag, None).await
}

/// A restarted process registers a fresh NodeId under an explicit relaunch
/// authorization that supersedes the prior incarnation.
async fn register_node_superseding(
    root: &DurabilityRoot,
    tag: u8,
    supersedes: Option<NodeId>,
) -> Result<NodeIncarnationProof, Box<dyn std::error::Error>> {
    let node = node_id(tag)?;
    let secret = BootstrapSecret::from_bytes([tag; 32]);
    let launch =
        LaunchBinding::new(&format!("s2-launch-{tag}")).map_err(|error| format!("{error:?}"))?;
    root.issue_node_bootstrap_authorization(&secret, &launch, supersedes)
        .await
        .map_err(|error| format!("{error:?}"))?;
    Ok(root
        .register_node_incarnation(&secret, &launch, node)
        .await
        .map_err(|error| format!("{error:?}"))?)
}

/// Control-plane issuance recorded before initialization (OPS-NODE-BOOT-01
/// D2). A refused issuance is left to the initialization to reject.
async fn issued_initialize(
    root: &DurabilityRoot,
    custody: &NodeIncarnationProof,
    provenance: FreshStoreProvenance,
    descriptor: DescriptorRegistration,
) -> Result<(), DurabilityError> {
    let _ = root
        .record_native_source_descriptor_issuance(
            &provenance.source_authority.clone(),
            descriptor.clone(),
            Some(provenance.clone()),
        )
        .await;
    root.initialize_native_admission_source(custody, provenance, descriptor)
        .await
}

/// Control-plane issuance of a later revision under the stored authority.
async fn issued_register(
    root: &DurabilityRoot,
    custody: &NodeIncarnationProof,
    descriptor: DescriptorRegistration,
) -> Result<(), DurabilityError> {
    let authority = match root.read_native_admission_source_registration().await {
        Ok(Some((stored, _))) => stored.source_authority,
        _ => provenance().source_authority,
    };
    let _ = root
        .record_native_source_descriptor_issuance(&authority, descriptor.clone(), None)
        .await;
    root.register_native_admission_descriptor(custody, descriptor)
        .await
}

fn provenance_for(source_authority: impl Into<String>) -> FreshStoreProvenance {
    FreshStoreProvenance {
        namespace: "store:one".into(),
        authorization: "owner:approved".into(),
        source_authority: source_authority.into(),
        initialized_at: 10,
    }
}

fn provenance() -> FreshStoreProvenance {
    provenance_for("platform")
}

fn descriptor_with_facts(
    revision: u64,
    facts: Vec<u8>,
    installed_at: i64,
) -> DescriptorRegistration {
    DescriptorRegistration {
        revision,
        facts,
        installed_at,
    }
}

fn descriptor(revision: u64, facts: u8, installed_at: i64) -> DescriptorRegistration {
    descriptor_with_facts(revision, vec![facts], installed_at)
}

fn account_observation(
    operation: NativeSourceOperation,
    source_revision: u64,
    decision_identity: impl Into<String>,
    observed_at: i64,
    semantic_facts: Vec<u8>,
) -> SourceObservation {
    SourceObservation {
        source_authority: "platform".into(),
        operation,
        subject: NativeSourceSubject::account_security(ACCOUNT_ID)
            .unwrap_or_else(|_| std::process::abort()),
        source_revision,
        decision_identity: decision_identity.into(),
        observed_at,
        semantic_facts,
    }
}

fn signing_observation(
    operation: NativeSourceOperation,
    issuer: &str,
    profile: &str,
    key_purpose: &str,
    key_id: &str,
    source_revision: u64,
) -> SourceObservation {
    SourceObservation {
        source_authority: "platform".into(),
        operation,
        subject: NativeSourceSubject::signing_trust(issuer, profile, key_purpose, key_id)
            .unwrap_or_else(|_| std::process::abort()),
        source_revision,
        decision_identity: format!("decision:{source_revision}"),
        observed_at: 100 + i64::try_from(source_revision.min(1000)).unwrap_or(1000),
        semantic_facts: vec![1],
    }
}

fn pending<'a>(
    publications: &'a [PendingPublication],
    binding: &[u8],
) -> Option<&'a PendingPublication> {
    publications
        .iter()
        .find(|publication| publication.operation_binding == binding)
}

#[test]
fn migration_declares_closed_bounded_nonrollback_store() {
    let migration = include_str!("../migrations/0004_native_admission_source.sql");
    for required in [
        "registration_id SMALLINT PRIMARY KEY CHECK (registration_id = 1)",
        "source_authority !~ '[^A-Za-z0-9._:/-]'",
        "octet_length(descriptor_facts) BETWEEN 1 AND 4096",
        "octet_length(operation_binding) BETWEEN 1 AND 16384",
        "octet_length(signing_key_id) BETWEEN 1 AND 64",
        "signing_key_id !~ '[^A-Za-z0-9._-]'",
        "slot_id SMALLINT NOT NULL CHECK (slot_id IN (1, 2))",
        "'ReadAccountSecurityV1'",
        "'ReadFreshSigningTrustV1'",
        "'ReadRecoveryAccountSecurityV2'",
        "'ReadRecoverySigningTrustV2'",
        "native source registration cannot roll back or be recreated",
        "native source floor cannot roll back or be rewritten",
        "native source canonical history is immutable",
        "custody_node_id UUID NOT NULL REFERENCES game_node_registrations (node_id)",
    ] {
        assert!(
            migration.contains(required),
            "missing migration constraint: {required}"
        );
    }
}

#[test]
fn typed_inputs_reject_unknown_namespace_and_invalid_account_ids() {
    assert!(matches!(
        NativeSourceOperation::parse("UnknownOperation"),
        Err(DurabilityError::Unavailable)
    ));
    for invalid in [
        "account:alternate",
        "00000000-0000-7000-8000-000000000000",
        "01890F4C-3B2A-7CC2-8D11-9A321B7C0001",
        "01890f4c-3b2a-4cc2-8d11-9a321b7c0001",
        "01890f4c3b2a7cc28d119a321b7c0001",
    ] {
        assert!(matches!(
            NativeSourceSubject::account_security(invalid),
            Err(DurabilityError::Unavailable)
        ));
    }
    assert!(
        NativeSourceSubject::signing_trust(
            FRESH_ISSUER,
            FRESH_PROFILE,
            "fresh_admission",
            "k".repeat(64),
        )
        .is_ok()
    );
    for invalid_key_id in [
        "k".repeat(65),
        "bad/key".into(),
        "bad key".into(),
        "é".into(),
    ] {
        assert!(matches!(
            NativeSourceSubject::signing_trust(
                FRESH_ISSUER,
                FRESH_PROFILE,
                "fresh_admission",
                invalid_key_id,
            ),
            Err(DurabilityError::Unavailable)
        ));
    }
}

#[test]
fn missing_registration_pending_read_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (admin_url, database_name, database_url) =
                create_database("native_source_missing_registration").await?;
            let result = async {
                migrate_postgres_17_6(&database_url).await?;
                let root = ready_root(&database_url).await?;
                let node = register_node(&root, 1).await?;
                assert!(matches!(
                    root.pending_native_source_publications(&node).await,
                    Err(DurabilityError::Unavailable)
                ));
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            cleanup_database(&admin_url, &database_name).await?;
            result
        })
}

#[test]
fn fresh_and_recovery_account_security_share_one_floor_across_restart()
-> Result<(), Box<dyn std::error::Error>> {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (admin_url, database_name, database_url) =
                create_database("native_source_floor").await?;
            let result = async {
                migrate_postgres_17_6(&database_url).await?;
                let root = ready_root(&database_url).await?;
                let node = register_node(&root, 1).await?;
                issued_initialize(&root, &node, provenance(), descriptor(1, 1, 10)).await?;
                issued_register(&root, &node, descriptor(2, 2, 20)).await?;
                issued_register(&root, &node, descriptor(2, 2, 20)).await?;
                assert!(matches!(
                    issued_register(&root, &node, descriptor(2, 3, 20)).await,
                    Err(DurabilityError::Unavailable)
                ));

                let fresh_10 = account_observation(
                    NativeSourceOperation::ReadAccountSecurityV1,
                    10,
                    "fresh:10",
                    100,
                    vec![1],
                );
                root.accept_native_source_observation(&node, fresh_10.clone())
                    .await?;
                root.accept_native_source_observation(&node, fresh_10.clone())
                    .await?;
                let recovery_11 = account_observation(
                    NativeSourceOperation::ReadRecoveryAccountSecurityV2,
                    11,
                    "recovery:11",
                    101,
                    vec![2],
                );
                root.accept_native_source_observation(&node, recovery_11.clone())
                    .await?;
                assert!(matches!(
                    root.accept_native_source_observation(&node, fresh_10).await,
                    Err(DurabilityError::Unavailable)
                ));
                let fresh_12 = account_observation(
                    NativeSourceOperation::ReadAccountSecurityV1,
                    12,
                    "fresh:12",
                    102,
                    vec![3],
                );
                root.accept_native_source_observation(&node, fresh_12)
                    .await?;
                assert!(matches!(
                    root.accept_native_source_observation(&node, recovery_11)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                drop(root);

                let restarted = ready_root(&database_url).await?;
                let maximum = account_observation(
                    NativeSourceOperation::ReadRecoveryAccountSecurityV2,
                    u64::MAX,
                    "recovery:max",
                    103,
                    vec![4],
                );
                restarted
                    .accept_native_source_observation(&node, maximum.clone())
                    .await?;
                restarted
                    .accept_native_source_observation(&node, maximum)
                    .await?;
                assert!(matches!(
                    restarted
                        .accept_native_source_observation(
                            &node,
                            account_observation(
                                NativeSourceOperation::ReadAccountSecurityV1,
                                u64::MAX - 1,
                                "fresh:lower",
                                104,
                                vec![5]
                            )
                        )
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            cleanup_database(&admin_url, &database_name).await?;
            result
        })
}

#[test]
fn wrong_pairings_fail_and_signing_key_switch_cannot_reset_floor()
-> Result<(), Box<dyn std::error::Error>> {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (admin_url, database_name, database_url) =
                create_database("native_source_signing").await?;
            let result = async {
                migrate_postgres_17_6(&database_url).await?;
                let root = ready_root(&database_url).await?;
                let node = register_node(&root, 1).await?;
                issued_initialize(&root, &node, provenance(), descriptor(1, 1, 10)).await?;

                let wrong = signing_observation(
                    NativeSourceOperation::ReadAccountSecurityV1,
                    FRESH_ISSUER,
                    FRESH_PROFILE,
                    "fresh_admission",
                    "kid-1",
                    1,
                );
                assert!(matches!(
                    root.accept_native_source_observation(&node, wrong).await,
                    Err(DurabilityError::Unavailable)
                ));
                let wrong_context = signing_observation(
                    NativeSourceOperation::ReadRecoverySigningTrustV2,
                    FRESH_ISSUER,
                    FRESH_PROFILE,
                    "fresh_admission",
                    "kid-1",
                    1,
                );
                assert!(matches!(
                    root.accept_native_source_observation(&node, wrong_context)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));

                let key_one = signing_observation(
                    NativeSourceOperation::ReadFreshSigningTrustV1,
                    FRESH_ISSUER,
                    FRESH_PROFILE,
                    "fresh_admission",
                    "kid-1",
                    10,
                );
                root.accept_native_source_observation(&node, key_one)
                    .await?;
                let key_two_lower = signing_observation(
                    NativeSourceOperation::ReadFreshSigningTrustV1,
                    FRESH_ISSUER,
                    FRESH_PROFILE,
                    "fresh_admission",
                    "kid-2",
                    1,
                );
                assert!(matches!(
                    root.accept_native_source_observation(&node, key_two_lower)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                let key_two_newer = signing_observation(
                    NativeSourceOperation::ReadFreshSigningTrustV1,
                    FRESH_ISSUER,
                    FRESH_PROFILE,
                    "fresh_admission",
                    "kid-2",
                    11,
                );
                root.accept_native_source_observation(&node, key_two_newer.clone())
                    .await?;
                root.accept_native_source_observation(&node, key_two_newer)
                    .await?;

                let maximum_key_id = "k".repeat(64);
                let maximum_key = signing_observation(
                    NativeSourceOperation::ReadFreshSigningTrustV1,
                    FRESH_ISSUER,
                    FRESH_PROFILE,
                    "fresh_admission",
                    &maximum_key_id,
                    12,
                );
                root.accept_native_source_observation(&node, maximum_key.clone())
                    .await?;
                root.accept_native_source_observation(&node, maximum_key)
                    .await?;

                let recovery = signing_observation(
                    NativeSourceOperation::ReadRecoverySigningTrustV2,
                    RECOVERY_ISSUER,
                    RECOVERY_PROFILE,
                    "existing_actor_recovery",
                    "recovery-kid",
                    1,
                );
                root.accept_native_source_observation(&node, recovery)
                    .await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            cleanup_database(&admin_url, &database_name).await?;
            result
        })
}

#[test]
fn resource_bounds_accept_max_and_reject_max_plus_one_before_retention()
-> Result<(), Box<dyn std::error::Error>> {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (admin_url, database_name, database_url) =
                create_database("native_source_bounds").await?;
            let result = async {
                migrate_postgres_17_6(&database_url).await?;
                let root = ready_root(&database_url).await?;
                let node = register_node(&root, 1).await?;
                let authority = "a".repeat(128);
                issued_initialize(
                    &root,
                    &node,
                    provenance_for(&authority),
                    descriptor_with_facts(1, vec![1; 4096], 10),
                )
                .await?;

                let mut at_max = account_observation(
                    NativeSourceOperation::ReadAccountSecurityV1,
                    1,
                    "d".repeat(256),
                    100,
                    vec![1; 8192],
                );
                at_max.source_authority = authority.clone();
                root.accept_native_source_observation(&node, at_max).await?;

                let mut long_authority = account_observation(
                    NativeSourceOperation::ReadAccountSecurityV1,
                    2,
                    "authority-long",
                    101,
                    vec![1],
                );
                long_authority.source_authority = "b".repeat(129);
                assert!(matches!(
                    root.accept_native_source_observation(&node, long_authority)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                let mut bad_authority = account_observation(
                    NativeSourceOperation::ReadAccountSecurityV1,
                    2,
                    "authority-bad",
                    101,
                    vec![1],
                );
                bad_authority.source_authority = "not allowed".into();
                assert!(matches!(
                    root.accept_native_source_observation(&node, bad_authority)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                assert!(matches!(
                    issued_register(&root, &node, descriptor_with_facts(2, vec![2; 4097], 20))
                        .await,
                    Err(DurabilityError::Unavailable)
                ));

                let mut long_decision = account_observation(
                    NativeSourceOperation::ReadAccountSecurityV1,
                    2,
                    "d".repeat(257),
                    101,
                    vec![1],
                );
                long_decision.source_authority = authority.clone();
                assert!(matches!(
                    root.accept_native_source_observation(&node, long_decision)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                let mut long_body = account_observation(
                    NativeSourceOperation::ReadAccountSecurityV1,
                    2,
                    "body-long",
                    101,
                    vec![1; 8193],
                );
                long_body.source_authority = authority;
                assert!(matches!(
                    root.accept_native_source_observation(&node, long_body)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            cleanup_database(&admin_url, &database_name).await?;
            result
        })
}

#[test]
fn descriptor_and_floor_rollback_are_rejected_without_losing_current_truth()
-> Result<(), Box<dyn std::error::Error>> {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (admin_url, database_name, database_url) = create_database("native_source_rollback").await?;
        let result = async {
            migrate_postgres_17_6(&database_url).await?;
            let root = ready_root(&database_url).await?;
            let node = register_node(&root, 1).await?;
            issued_initialize(&root, &node, provenance(), descriptor(2, 2, 20)).await?;
            assert!(matches!(
                issued_register(&root, &node, descriptor(1, 1, 10)).await,
                Err(DurabilityError::Unavailable)
            ));
            let base = account_observation(
                NativeSourceOperation::ReadAccountSecurityV1, 20, "fresh:20", 200, vec![1]);
            let advance = account_observation(
                NativeSourceOperation::ReadAccountSecurityV1, 21, "fresh:21", 201, vec![2]);
            root.accept_native_source_observation(&node, base.clone()).await?;

            let mut fault = sqlx::PgConnection::connect(&database_url).await?;
            sqlx::query(
                "CREATE FUNCTION reject_native_source_floor_advance() RETURNS trigger \
                 LANGUAGE plpgsql AS $$ BEGIN IF NEW.source_revision = 21 THEN \
                 RAISE EXCEPTION 'forced native source rollback' USING ERRCODE = '23514'; \
                 END IF; RETURN NEW; END; $$",
            ).execute(&mut fault).await?;
            sqlx::query(
                "CREATE TRIGGER reject_native_source_floor_advance BEFORE UPDATE ON \
                 game_durability_native_source_floors FOR EACH ROW EXECUTE FUNCTION \
                 reject_native_source_floor_advance()",
            ).execute(&mut fault).await?;
            assert!(matches!(
                root.accept_native_source_observation(&node, advance.clone()).await,
                Err(DurabilityError::Database(_))
            ));
            sqlx::query("DROP TRIGGER reject_native_source_floor_advance ON game_durability_native_source_floors")
                .execute(&mut fault).await?;
            sqlx::query("DROP FUNCTION reject_native_source_floor_advance()")
                .execute(&mut fault).await?;
            fault.close().await?;
            drop(root);

            let restarted = ready_root(&database_url).await?;
            restarted.accept_native_source_observation(&node, base).await?;
            restarted.accept_native_source_observation(&node, advance).await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        cleanup_database(&admin_url, &database_name).await?;
        result
    })
}

#[test]
fn lost_response_two_max_slots_exact_clear_and_reuse_survive_restart()
-> Result<(), Box<dyn std::error::Error>> {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (admin_url, database_name, database_url) =
                create_database("native_source_slots").await?;
            let result = async {
                migrate_postgres_17_6(&database_url).await?;
                let root = ready_root(&database_url).await?;
                let node = register_node(&root, 1).await?;
                issued_initialize(&root, &node, provenance(), descriptor(1, 1, 10)).await?;
                let first = vec![1; 16_384];
                let second = vec![2; 16_384];
                let third = vec![3];
                let first_slot = root
                    .checkpoint_native_source_publication(&node, first.clone(), 300)
                    .await?;
                drop(root);

                let restarted = ready_root(&database_url).await?;
                assert_eq!(
                    restarted
                        .checkpoint_native_source_publication(&node, first.clone(), 999)
                        .await?,
                    first_slot
                );
                let after_replay = restarted.pending_native_source_publications(&node).await?;
                let replayed = pending(&after_replay, &first).ok_or("missing replayed slot")?;
                assert_eq!(replayed.checkpointed_at, 300);
                let second_slot = restarted
                    .checkpoint_native_source_publication(&node, second.clone(), 301)
                    .await?;
                let aggregate: usize = restarted
                    .pending_native_source_publications(&node)
                    .await?
                    .iter()
                    .map(|publication| publication.operation_binding.len())
                    .sum();
                assert_eq!(aggregate, 32_768);
                assert!(matches!(
                    restarted
                        .checkpoint_native_source_publication(&node, third.clone(), 302)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                assert!(matches!(
                    restarted
                        .checkpoint_native_source_publication(&node, vec![4; 16_385], 303)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                assert!(matches!(
                    restarted
                        .clear_native_source_publication(&node, first_slot, third.clone())
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                restarted
                    .clear_native_source_publication(&node, first_slot, first)
                    .await?;
                drop(restarted);

                let reconciled = ready_root(&database_url).await?;
                let remaining = reconciled.pending_native_source_publications(&node).await?;
                assert_eq!(remaining.len(), 1);
                assert_eq!(remaining[0].slot_id, second_slot);
                assert_eq!(remaining[0].operation_binding, second);
                assert_eq!(
                    reconciled
                        .checkpoint_native_source_publication(&node, third, 302)
                        .await?,
                    first_slot
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            cleanup_database(&admin_url, &database_name).await?;
            result
        })
}

async fn source_state(
    database_url: &str,
) -> Result<(String, i64, i64), Box<dyn std::error::Error>> {
    let mut connection = sqlx::PgConnection::connect(database_url).await?;
    let state: (String, i64, i64) = sqlx::query_as(
        "SELECT (SELECT coalesce(max(source_revision), 0)::text FROM game_durability_native_source_floors), \
                (SELECT count(*) FROM game_durability_native_source_publication_slots WHERE operation_binding IS NOT NULL), \
                (SELECT count(*) FROM game_durability_native_source_descriptor_history)",
    )
    .fetch_one(&mut connection)
    .await?;
    connection.close().await?;
    Ok(state)
}

async fn every_mutation_is_fenced(
    root: &DurabilityRoot,
    custody: &NodeIncarnationProof,
    database_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let before = source_state(database_url).await?;
    assert!(matches!(
        root.initialize_native_admission_source(custody, provenance(), descriptor(1, 1, 10))
            .await,
        Err(DurabilityError::Unavailable)
    ));
    assert!(matches!(
        root.register_native_admission_descriptor(custody, descriptor(9, 9, 90))
            .await,
        Err(DurabilityError::Unavailable)
    ));
    assert!(matches!(
        root.accept_native_source_observation(
            custody,
            account_observation(
                NativeSourceOperation::ReadAccountSecurityV1,
                90,
                "stale:90",
                900,
                vec![9]
            ),
        )
        .await,
        Err(DurabilityError::Unavailable)
    ));
    assert!(matches!(
        root.checkpoint_native_source_publication(custody, vec![9], 900)
            .await,
        Err(DurabilityError::Unavailable)
    ));
    assert!(matches!(
        root.clear_native_source_publication(custody, 1, vec![1])
            .await,
        Err(DurabilityError::Unavailable)
    ));
    assert!(matches!(
        root.pending_native_source_publications(custody).await,
        Err(DurabilityError::Unavailable)
    ));
    let subject = RecoveryEvidenceSubject::new(ACCOUNT_ID, "recovery-1")?;
    assert!(matches!(
        root.verify_registered_recovery(
            custody,
            &subject,
            &recovery_token("recovery-1", 100),
            &recovery_bindings()?
        )
        .await,
        Err(DurabilityError::Unavailable)
    ));
    assert_eq!(
        source_state(database_url).await?,
        before,
        "fenced caller changed source state"
    );
    Ok(())
}

#[test]
fn replaced_stale_and_unregistered_incarnations_cannot_mutate_or_establish_custody()
-> Result<(), Box<dyn std::error::Error>> {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (admin_url, database_name, database_url) =
                create_database("native_source_custody").await?;
            let result = async {
                migrate_postgres_17_6(&database_url).await?;
                let first_root = ready_root(&database_url).await?;
                // An unregistered incarnation cannot initialize custody.
                let unregistered = NodeIncarnationProof::new(
                    NodeRegistrationFact::new(node_id(9)?, 1),
                    BootstrapSecret::from_bytes([9; 32]),
                );
                assert!(matches!(
                    issued_initialize(&first_root, &unregistered, provenance(), descriptor(1, 1, 10))
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                let first = register_node(&first_root, 1).await?;
                issued_initialize(&first_root, &first, provenance(), descriptor(1, 1, 10))
                    .await?;
                first_root
                    .accept_native_source_observation(
                        &first,
                        account_observation(NativeSourceOperation::ReadAccountSecurityV1, 1, "fresh:1", 100, vec![1]),
                    )
                    .await?;
                let slot = first_root
                    .checkpoint_native_source_publication(&first, vec![1], 100)
                    .await?;
                assert_eq!(slot, 1);
                // A wrong-incarnation fact (old receipt shape), an unregistered
                // incarnation and the holder's public fact without its own secret
                // are not custody.
                let wrong_incarnation = NodeIncarnationProof::new(
                    NodeRegistrationFact::new(first.fact().node_id(), first.fact().registration_revision() + 1),
                    BootstrapSecret::from_bytes([1; 32]),
                );
                let public_fact_only = NodeIncarnationProof::new(first.fact(), BootstrapSecret::from_bytes([7; 32]));
                for impostor in [&wrong_incarnation, &unregistered, &public_fact_only] {
                    every_mutation_is_fenced(&first_root, impostor, &database_url).await?;
                }

                // Restart/replacement: a second root registers a fresh incarnation
                // that supersedes the first. The old process may keep running.
                let second_root = ready_root(&database_url).await?;
                let second = register_node_superseding(&second_root, 2, Some(first.fact().node_id())).await?;
                for root in [&first_root, &second_root] {
                    every_mutation_is_fenced(root, &first, &database_url).await?;
                }
                // Persisted source state alone does not transfer custody.
                every_mutation_is_fenced(&second_root, &second, &database_url).await?;
                assert!(matches!(
                    first_root.claim_native_admission_source_custody(&first).await,
                    Err(DurabilityError::Unavailable)
                ));
                // Knowing the successor's public identity does not let the old
                // process claim or use custody.
                let stolen = NodeIncarnationProof::new(second.fact(), BootstrapSecret::from_bytes([1; 32]));
                assert!(matches!(
                    first_root.claim_native_admission_source_custody(&stolen).await,
                    Err(DurabilityError::Unavailable)
                ));
                second_root.claim_native_admission_source_custody(&second).await?;
                second_root.claim_native_admission_source_custody(&second).await?;
                every_mutation_is_fenced(&first_root, &stolen, &database_url).await?;

                // A different current incarnation cannot seize live custody.
                let third_root = ready_root(&database_url).await?;
                let third = register_node(&third_root, 3).await?;
                assert!(matches!(
                    third_root.claim_native_admission_source_custody(&third).await,
                    Err(DurabilityError::Unavailable)
                ));
                every_mutation_is_fenced(&third_root, &third, &database_url).await?;

                // The current holder resumes exactly the retained state.
                let pending_after = second_root.pending_native_source_publications(&second).await?;
                assert_eq!(pending_after.len(), 1);
                assert_eq!(pending_after[0].operation_binding, vec![1]);
                second_root.clear_native_source_publication(&second, slot, vec![1]).await?;
                second_root
                    .accept_native_source_observation(
                        &second,
                        account_observation(NativeSourceOperation::ReadAccountSecurityV1, 2, "fresh:2", 101, vec![2]),
                    )
                    .await?;
                issued_register(&second_root, &second, descriptor(2, 2, 20))
                    .await?;
                assert_eq!(source_state(&database_url).await?, ("2".into(), 0, 2));

                // Rewriting persisted custody back to the replaced incarnation
                // cannot revive it: currentness is proven in every transaction.
                let mut admin = sqlx::PgConnection::connect(&database_url).await?;
                sqlx::query(
                    "UPDATE game_durability_native_source_registration \
                     SET custody_node_id = encode($1, 'hex')::uuid, custody_registration_revision = $2::text::numeric(20,0)",
                )
                .bind(first.fact().node_id().as_bytes().as_slice())
                .bind(first.fact().registration_revision().to_string())
                .execute(&mut admin)
                .await?;
                admin.close().await?;
                every_mutation_is_fenced(&first_root, &first, &database_url).await?;
                every_mutation_is_fenced(&second_root, &second, &database_url).await?;
                second_root.claim_native_admission_source_custody(&second).await?;
                second_root.pending_native_source_publications(&second).await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            cleanup_database(&admin_url, &database_name).await?;
            result
        })
}

#[test]
fn revoke_serializes_with_current_custody_and_later_mutations_fail_atomically()
-> Result<(), Box<dyn std::error::Error>> {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (admin_url, database_name, database_url) =
                create_database("native_source_revoke").await?;
            let result = async {
                migrate_postgres_17_6(&database_url).await?;
                let root = ready_root(&database_url).await?;
                let node = register_node(&root, 1).await?;
                issued_initialize(&root, &node, provenance(), descriptor(1, 1, 10))
                    .await?;
                // An in-flight fenced transaction holds the current-incarnation
                // share lock; revocation waits for it instead of interleaving.
                let mut inflight = sqlx::PgConnection::connect(&database_url).await?;
                sqlx::query("BEGIN").execute(&mut inflight).await?;
                sqlx::query(
                    "SELECT game_node_require_current(encode($1, 'hex')::uuid, $2::text::numeric(20,0), $3)",
                )
                .bind(node.fact().node_id().as_bytes().as_slice())
                .bind(node.fact().registration_revision().to_string())
                .bind([1_u8; 32].as_slice())
                .execute(&mut inflight)
                .await?;
                let revoker = ready_root(&database_url).await?;
                let fact = node.fact();
                let revocation = tokio::spawn(async move { revoker.revoke_node_registration(fact).await });
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                assert!(!revocation.is_finished(), "revoke did not serialize with in-flight custody");
                sqlx::query("COMMIT").execute(&mut inflight).await?;
                inflight.close().await?;
                revocation.await?.map_err(|error| format!("{error:?}"))?;
                every_mutation_is_fenced(&root, &node, &database_url).await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            cleanup_database(&admin_url, &database_name).await?;
            result
        })
}

fn recovery_now() -> Result<i64, Box<dyn std::error::Error>> {
    Ok(i64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    )?)
}

#[test]
fn recovery_subject_rejects_unbounded_inputs_before_owner_reads() {
    assert!(RecoveryEvidenceSubject::new("x".repeat(65_537), "key").is_err());
    assert!(RecoveryEvidenceSubject::new(ACCOUNT_ID, "k".repeat(65)).is_err());
    assert!(RecoveryEvidenceSubject::new(ACCOUNT_ID, "key").is_ok());
}

fn recovery_bindings() -> Result<RecoveryCurrentEvidence, Box<dyn std::error::Error>> {
    Ok(RecoveryCurrentEvidence {
        account_id: ACCOUNT_ID.into(),
        character_id: CharacterId::decode(node_id(2)?.as_bytes()).map_err(|e| format!("{e:?}"))?,
        world_id: WorldId::decode(node_id(3)?.as_bytes()).map_err(|e| format!("{e:?}"))?,
        ruleset_revision: "rules-1".into(),
        content_revision: "content-1".into(),
        map_revision: "map-1".into(),
        world_policy_revision: "policy-1".into(),
    })
}

fn recovery_token(key_id: &str, now: i64) -> String {
    let header = serde_json::json!({"alg":"Ed25519","kid":key_id,"typ":"oteryn-recovery+jwt"});
    // Independent fixture bindings, not copied from a stored S2 response.
    let payload = serde_json::json!({
        "iss":RECOVERY_ISSUER,"aud":"urn:oteryn:game:recovery",
        "iat":now,"nbf":now,"exp":now+10,
        "jti":URL_SAFE_NO_PAD.encode([8;32]),"profile":RECOVERY_PROFILE,
        "purpose":"existing_actor_recovery","attempt_ref":"01890f4c-3b2a-7cc2-8d11-9a321b7c0004",
        "account_id":ACCOUNT_ID,"character_id":"01890f4c-3b2a-7c02-8d11-9a321b7c0002",
        "world_id":"01890f4c-3b2a-7c03-8d11-9a321b7c0003",
        "account_security_generation":"1","protocol_major":1,"transport_profile":1,
        "ruleset_revision":"rules-1","content_revision":"content-1","map_revision":"map-1","world_policy_revision":"policy-1"
    });
    let input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header.to_string()),
        URL_SAFE_NO_PAD.encode(payload.to_string())
    );
    format!(
        "{input}.{}",
        URL_SAFE_NO_PAD.encode(
            SigningKey::from_bytes(&[23; 32])
                .sign(input.as_bytes())
                .to_bytes()
        )
    )
}

fn recovery_account(revision: u64, now: i64, allowed: bool) -> SourceObservation {
    let body = serde_json::json!({
        "version":2,"operation":"ReadRecoveryAccountSecurityV2","result":"observed",
        "source_authority":"platform","source_revision":revision.to_string(),"decision_identity":revision.to_string(),
        "source_observed_at":now.to_string(),"clock_uncertainty_seconds":"0",
        "account_id":ACCOUNT_ID,"purpose":"platform_security","scope":"existing_actor_recovery",
        "allowed":allowed,"minimum_valid_generation":"1"
    });
    account_observation(
        NativeSourceOperation::ReadRecoveryAccountSecurityV2,
        revision,
        revision.to_string(),
        now,
        body.to_string().into_bytes(),
    )
}

fn recovery_trust(revision: u64, now: i64, key_id: &str) -> SourceObservation {
    let mut observation = signing_observation(
        NativeSourceOperation::ReadRecoverySigningTrustV2,
        RECOVERY_ISSUER,
        RECOVERY_PROFILE,
        "existing_actor_recovery",
        key_id,
        revision,
    );
    observation.decision_identity = revision.to_string();
    observation.observed_at = now;
    observation.semantic_facts = serde_json::json!({
        "version":2,"operation":"ReadRecoverySigningTrustV2","result":"observed",
        "source_authority":"platform","source_revision":revision.to_string(),"decision_identity":revision.to_string(),
        "source_observed_at":now.to_string(),"clock_uncertainty_seconds":"0",
        "issuer":RECOVERY_ISSUER,"profile":RECOVERY_PROFILE,"key_purpose":"existing_actor_recovery","key_id":key_id,
        "trusted":true,"public_key":URL_SAFE_NO_PAD.encode(SigningKey::from_bytes(&[23;32]).verifying_key().to_bytes())
    }).to_string().into_bytes();
    observation
}

#[test]
fn recovery_verification_resolves_current_owners_replay_restart_and_purpose_floors()
-> Result<(), Box<dyn std::error::Error>> {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (admin_url, name, url) = create_database("recovery_current").await?;
            let result = async {
                migrate_postgres_17_6(&url).await?;
                let root = ready_root(&url).await?;
                let node = register_node(&root, 1).await?;
                let now = recovery_now()?;
                let current = recovery_bindings()?;
                let subject = RecoveryEvidenceSubject::new(ACCOUNT_ID, "recovery-1")?;
                let token = recovery_token("recovery-1", now);
                let account = recovery_account(1, now, true);
                let trust = recovery_trust(1, now, "recovery-1");
                assert!(matches!(
                    root.verify_registered_recovery(&node, &subject, &token, &current)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                issued_initialize(&root, &node, provenance(), descriptor(1, 1, 10)).await?;
                assert!(matches!(
                    root.verify_registered_recovery(&node, &subject, &token, &current)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                root.accept_native_source_observation(&node, account.clone())
                    .await?;
                assert!(matches!(
                    root.verify_registered_recovery(&node, &subject, &token, &current)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                root.accept_native_source_observation(&node, trust.clone())
                    .await?;
                let verified = root
                    .verify_registered_recovery(&node, &subject, &token, &current)
                    .await?
                    .map_err(|e| format!("{e:?}"))?;
                assert_eq!(verified.security().provenance.publication_revision, 1);
                assert_eq!(verified.security().provenance.source_observed_at, now);
                let mut changed = account.clone();
                let mut body: serde_json::Value = serde_json::from_slice(&changed.semantic_facts)?;
                body["allowed"] = serde_json::json!(false);
                changed.semantic_facts = body.to_string().into_bytes();
                assert!(matches!(
                    root.accept_native_source_observation(&node, changed).await,
                    Err(DurabilityError::Unavailable)
                ));
                root.accept_native_source_observation(&node, account)
                    .await?;
                root.accept_native_source_observation(&node, trust).await?;
                let replay = root
                    .revalidate_registered_recovery(&node, &verified, &current)
                    .await?
                    .map_err(|e| format!("{e:?}"))?;
                assert_eq!(replay.security(), verified.security());
                assert_eq!(replay.signing(), verified.signing());
                // Partial owner loss is resolved again, never reconstructed
                // from retained facts. Restore exact immutable history bytes
                // after each independent missing-floor case.
                let mut owner = sqlx::PgConnection::connect(&url).await?;
                sqlx::query("ALTER TABLE game_durability_native_source_floors DISABLE TRIGGER USER").execute(&mut owner).await?;
                for operation in ["ReadRecoveryAccountSecurityV2","ReadRecoverySigningTrustV2"] {
                    sqlx::query("DELETE FROM game_durability_native_source_floors WHERE operation=$1").bind(operation).execute(&mut owner).await?;
                    assert!(matches!(root.revalidate_registered_recovery(&node,&verified,&current).await,Err(DurabilityError::Unavailable)));
                    sqlx::query("INSERT INTO game_durability_native_source_floors SELECT registration_id,source_authority,operation,floor_subject,observation_subject,signing_key_id,source_revision,decision_identity,observed_at,semantic_facts FROM game_durability_native_source_observation_history WHERE operation=$1 AND source_revision=1").bind(operation).execute(&mut owner).await?;
                }
                sqlx::query("ALTER TABLE game_durability_native_source_floors ENABLE TRIGGER USER").execute(&mut owner).await?;
                owner.close().await?;
                let reload = ready_root(&url).await?;
                assert!(
                    reload
                        .revalidate_registered_recovery(&node, &verified, &current)
                        .await?
                        .is_ok()
                );
                // N+1 denial must invalidate an escaped historical verified value.
                root.accept_native_source_observation(&node, recovery_account(2, now, false))
                    .await?;
                assert!(matches!(
                    reload
                        .revalidate_registered_recovery(&node, &verified, &current)
                        .await?,
                    Err(Fnd04ConsumerError::RecoverySecurityStateRevoked)
                ));
                root.accept_native_source_observation(&node, recovery_account(3, now, true))
                    .await?;
                let newer = root
                    .revalidate_registered_recovery(&node, &verified, &current)
                    .await?
                    .map_err(|e| format!("{e:?}"))?;
                assert_eq!(newer.security().provenance.publication_revision, 3);
                // Same allowed facts under Fresh still supersede the shared Account floor.
                let mut fresh = recovery_account(4, now, true);
                fresh.operation = NativeSourceOperation::ReadAccountSecurityV1;
                let mut body: serde_json::Value = serde_json::from_slice(&fresh.semantic_facts)?;
                body["version"] = serde_json::json!(1);
                body["operation"] = serde_json::json!("ReadAccountSecurityV1");
                body["scope"] = serde_json::json!("fresh_admission");
                fresh.semantic_facts = body.to_string().into_bytes();
                root.accept_native_source_observation(&node, fresh).await?;
                assert!(matches!(
                    root.revalidate_registered_recovery(&node, &newer, &current)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                root.accept_native_source_observation(&node, recovery_account(5, now, true))
                    .await?;
                let mut revoked_key = recovery_trust(2, now, "recovery-1");
                let mut body: serde_json::Value =
                    serde_json::from_slice(&revoked_key.semantic_facts)?;
                body["trusted"] = serde_json::json!(false);
                revoked_key.semantic_facts = body.to_string().into_bytes();
                root.accept_native_source_observation(&node, revoked_key)
                    .await?;
                assert!(matches!(
                    root.verify_registered_recovery(&node, &subject, &token, &current)
                        .await?,
                    Err(Fnd04ConsumerError::RecoveryAuthenticationFailed)
                ));
                root.accept_native_source_observation(&node, recovery_trust(3, now, "other-key"))
                    .await?;
                assert!(matches!(
                    root.verify_registered_recovery(&node, &subject, &token, &current)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                // Explicit later trust observation restores only the queried key.
                root.accept_native_source_observation(&node, recovery_trust(4, now, "recovery-1"))
                    .await?;
                assert!(
                    root.verify_registered_recovery(&node, &subject, &token, &current)
                        .await?
                        .is_ok()
                );
                let successor =
                    register_node_superseding(&reload, 2, Some(node.fact().node_id())).await?;
                assert!(matches!(
                    root.revalidate_registered_recovery(&node, &verified, &current)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                assert!(matches!(
                    reload
                        .verify_registered_recovery(&successor, &subject, &token, &current)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                reload
                    .claim_native_admission_source_custody(&successor)
                    .await?;
                let observed = recovery_now()?;
                reload
                    .accept_native_source_observation(
                        &successor,
                        recovery_account(6, observed, true),
                    )
                    .await?;
                reload
                    .accept_native_source_observation(
                        &successor,
                        recovery_trust(5, observed, "recovery-1"),
                    )
                    .await?;
                let after_restart = reload
                    .revalidate_registered_recovery(&successor, &verified, &current)
                    .await?
                    .map_err(|e| format!("{e:?}"))?;
                let mut advanced_generation = recovery_account(7, observed, true);
                let mut body: serde_json::Value =
                    serde_json::from_slice(&advanced_generation.semantic_facts)?;
                body["minimum_valid_generation"] = serde_json::json!("2");
                advanced_generation.semantic_facts = body.to_string().into_bytes();
                reload
                    .accept_native_source_observation(&successor, advanced_generation)
                    .await?;
                assert!(matches!(
                    reload
                        .revalidate_registered_recovery(&successor, &after_restart, &current)
                        .await?,
                    Err(Fnd04ConsumerError::RecoverySecurityStateRevoked)
                ));
                reload
                    .revoke_node_registration(successor.fact())
                    .await
                    .map_err(|e| format!("{e:?}"))?;
                assert!(matches!(
                    reload
                        .verify_registered_recovery(&successor, &subject, &token, &current)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                // Credential reads never create session/lease/controller authority.
                let mut admin = sqlx::PgConnection::connect(&url).await?;
                let sessions: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_durability_reconnect_sessions")
                        .fetch_one(&mut admin)
                        .await?;
                assert_eq!(sessions, 0);
                admin.close().await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            cleanup_database(&admin_url, &name).await?;
            result
        })
}

#[test]
fn recovery_redecode_rejects_independent_body_history_and_time_mutations()
-> Result<(), Box<dyn std::error::Error>> {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let(admin_url,name,url)=create_database("recovery_provenance").await?;
        let result=async {
            migrate_postgres_17_6(&url).await?;
            let root=ready_root(&url).await?;let node=register_node(&root,1).await?;
            issued_initialize(&root,&node,provenance(),descriptor(1,1,10)).await?;
            let now=recovery_now()?;let current=recovery_bindings()?;
            let subject=RecoveryEvidenceSubject::new(ACCOUNT_ID,"recovery-1")?;
            let token=recovery_token("recovery-1",now);
            root.accept_native_source_observation(&node,recovery_trust(1,now,"recovery-1")).await?;
            // Each case changes one source invariant, with a valid trust/token/binding.
            for (index,field,value) in [
                (1,"account_id",serde_json::json!("01890f4c-3b2a-7cc2-8d11-9a321b7c0002")),
                (2,"source_authority",serde_json::json!("other-authority")),
                (3,"scope",serde_json::json!("fresh_admission")),
                (4,"source_revision",serde_json::json!("99")),
                (5,"source_observed_at",serde_json::json!((now-1).to_string())),
            ] {
                let mut observation=recovery_account(index,now,true);
                let mut body:serde_json::Value=serde_json::from_slice(&observation.semantic_facts)?;
                body[field]=value;observation.semantic_facts=body.to_string().into_bytes();
                root.accept_native_source_observation(&node,observation).await?;
                assert!(matches!(root.verify_registered_recovery(&node,&subject,&token,&current).await,Err(DurabilityError::InvalidStoredState)),"{field}");
            }
            for (revision,time,uncertainty) in [(6,now-6,"0"),(7,now+30,"0"),(8,now,"18446744073709551615")] {
                let mut observation=recovery_account(revision,time,true);
                let mut body:serde_json::Value=serde_json::from_slice(&observation.semantic_facts)?;
                body["clock_uncertainty_seconds"]=serde_json::json!(uncertainty);
                observation.semantic_facts=body.to_string().into_bytes();
                root.accept_native_source_observation(&node,observation).await?;
                assert!(matches!(root.verify_registered_recovery(&node,&subject,&token,&current).await?,Err(Fnd04ConsumerError::RecoverySecurityEvidenceStale)));
            }
            root.accept_native_source_observation(&node,recovery_account(9,now,true)).await?;
            let mut wrong=current.clone();wrong.account_id="01890f4c-3b2a-7cc2-8d11-9a321b7c0002".into();
            assert!(matches!(root.verify_registered_recovery(&node,&subject,&token,&wrong).await?,Err(Fnd04ConsumerError::RecoveryBindingMismatch)));
            // Owner-only corruption models a partial restore, not a full DB rollback.
            let mut admin=sqlx::PgConnection::connect(&url).await?;
            sqlx::query("ALTER TABLE game_durability_native_source_observation_history DISABLE TRIGGER USER").execute(&mut admin).await?;
            sqlx::query("DELETE FROM game_durability_native_source_observation_history WHERE operation='ReadRecoveryAccountSecurityV2' AND source_revision=9").execute(&mut admin).await?;
            assert!(matches!(root.verify_registered_recovery(&node,&subject,&token,&current).await,Err(DurabilityError::InvalidStoredState)));
            admin.close().await?;
            Ok::<(),Box<dyn std::error::Error>>(())
        }.await;
        cleanup_database(&admin_url,&name).await?;result
    })
}

#[test]
fn recovery_clock_is_sampled_after_registration_wait() -> Result<(), Box<dyn std::error::Error>> {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (admin_url, name, url) = create_database("recovery_clock_wait").await?;
        let result = async {
            migrate_postgres_17_6(&url).await?;
            let root = ready_root(&url).await?;
            let node = register_node(&root, 1).await?;
            issued_initialize(&root, &node, provenance(), descriptor(1,1,10)).await?;
            let mut blocker = sqlx::PgConnection::connect(&url).await?;
            let current = recovery_bindings()?;
            let subject = RecoveryEvidenceSubject::new(ACCOUNT_ID,"recovery-1")?;
            let mut oversized = current.clone();oversized.ruleset_revision = "x".repeat(65_537);
            assert!(matches!(root.verify_registered_recovery(&node,&subject,"inert",&oversized).await?,Err(Fnd04ConsumerError::RecoveryMalformed)));
            let mut qualified = false;
            // Only an observed lock wait still inside the valid fixture second
            // qualifies. Bounded alignment retries avoid credit for an already
            // expired setup; no sleep or replacement production clock is used.
            for revision in 1..=3 {
                let previous = recovery_now()?;
                while recovery_now()? == previous { tokio::task::yield_now().await; }
                let now = recovery_now()?;
                root.accept_native_source_observation(&node,recovery_account(revision,now-5,true)).await?;
                root.accept_native_source_observation(&node,recovery_trust(revision,now,"recovery-1")).await?;
                sqlx::query("BEGIN").execute(&mut blocker).await?;
                sqlx::query("SELECT registration_id FROM game_durability_native_source_registration WHERE registration_id=1 FOR UPDATE").execute(&mut blocker).await?;
                let reader=root.clone();let proof=node.clone();let binding=current.clone();let selected=subject.clone();
                let token = recovery_token("recovery-1",now);
                let pending = tokio::spawn(async move {
                    reader.verify_registered_recovery(&proof,&selected,&token,&binding).await
                });
                tokio::time::timeout(durability::DB_PASS_DEADLINE, async {
                    loop {
                        let waiting:bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND pg_backend_pid()=ANY(pg_blocking_pids(pid)))").fetch_one(&mut blocker).await?;
                        if waiting { break; }
                        tokio::task::yield_now().await;
                    }
                    Ok::<(),sqlx::Error>(())
                }).await??;
                assert!(!pending.is_finished());
                if recovery_now()? != now {
                    sqlx::query("COMMIT").execute(&mut blocker).await?;
                    let _ = pending.await?;
                    continue;
                }
                // Before-SQL sampling would accept at this exact deadline.
                // Cross it while that actual request is demonstrably blocked.
                while recovery_now()? <= now { tokio::task::yield_now().await; }
                sqlx::query("COMMIT").execute(&mut blocker).await?;
                let disposition = pending.await??;
                assert!(matches!(disposition,Err(Fnd04ConsumerError::RecoverySecurityEvidenceStale)));
                qualified = true;
                break;
            }
            assert!(qualified,"no fixture remained valid during observed lock wait");
            blocker.close().await?;
            Ok::<(),Box<dyn std::error::Error>>(())
        }.await;
        cleanup_database(&admin_url,&name).await?;result
    })
}

#[test]
fn recovery_verification_and_s2_denial_serialize_at_registration()
-> Result<(), Box<dyn std::error::Error>> {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (admin_url,name,url)=create_database("recovery_s2_race").await?;
        let result=async {
            migrate_postgres_17_6(&url).await?;
            let reader=ready_root(&url).await?;let writer=ready_root(&url).await?;
            let node=register_node(&reader,1).await?;
            issued_initialize(&reader,&node,provenance(),descriptor(1,1,10)).await?;
            let now=recovery_now()?;
            reader.accept_native_source_observation(&node,recovery_account(1,now,true)).await?;
            reader.accept_native_source_observation(&node,recovery_trust(1,now,"recovery-1")).await?;
            let current=recovery_bindings()?;let subject=RecoveryEvidenceSubject::new(ACCOUNT_ID,"recovery-1")?;
            let token=recovery_token("recovery-1",now);
            let mut blocker=sqlx::PgConnection::connect(&url).await?;
            sqlx::query("BEGIN").execute(&mut blocker).await?;
            sqlx::query("SELECT registration_id FROM game_durability_native_source_registration WHERE registration_id=1 FOR UPDATE").execute(&mut blocker).await?;
            let proof=node.clone();let binding=current.clone();let selected=subject.clone();let grant=token.clone();let checking=reader.clone();
            let verification=tokio::spawn(async move { checking.verify_registered_recovery(&proof,&selected,&grant,&binding).await });
            let proof=node.clone();
            let acknowledgement=tokio::spawn(async move { writer.accept_native_source_observation(&proof,recovery_account(2,now,false)).await });
            tokio::time::timeout(durability::DB_PASS_DEADLINE,async {
                loop {
                    // The second request can wait on the first request's tuple
                    // lock. Both chains must reach this registration-lock holder.
                    let waiting:i64=sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity activity WHERE datname=current_database() AND EXISTS (SELECT 1 FROM unnest(pg_blocking_pids(activity.pid)) AS dependency(blocker_pid) WHERE dependency.blocker_pid=pg_backend_pid() OR pg_backend_pid()=ANY(pg_blocking_pids(dependency.blocker_pid)))").fetch_one(&mut blocker).await?;
                    if waiting==2 {break;}
                    tokio::task::yield_now().await;
                }
                Ok::<(),sqlx::Error>(())
            }).await??;
            assert!(!verification.is_finished());assert!(!acknowledgement.is_finished());
            sqlx::query("COMMIT").execute(&mut blocker).await?;
            let observed=verification.await??;
            acknowledgement.await??;
            // Either legal serialization is accepted: historical allow before
            // acknowledgement or denial after it. Never allow after both finish.
            assert!(observed.is_ok() || matches!(observed,Err(Fnd04ConsumerError::RecoverySecurityStateRevoked)));
            assert!(matches!(reader.verify_registered_recovery(&node,&subject,&token,&current).await?,Err(Fnd04ConsumerError::RecoverySecurityStateRevoked)));
            blocker.close().await?;
            Ok::<(),Box<dyn std::error::Error>>(())
        }.await;
        cleanup_database(&admin_url,&name).await?;result
    })
}
