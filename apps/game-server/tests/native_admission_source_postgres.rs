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

use durability::native_admission_source::{
    DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
    PendingPublication, SourceObservation,
};
use durability::runtime_scope_assignment::{
    BootstrapSecret, LaunchBinding, NodeIncarnationProof, NodeRegistrationFact,
};
use durability::{DurabilityError, DurabilityRoot};
use foundation::NodeId;
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
