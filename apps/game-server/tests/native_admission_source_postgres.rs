// Include the production durability implementation in this dedicated PostgreSQL
// target. Ordinary workspace runs may skip when the routed PostgreSQL service is
// absent; only configured PostgreSQL 17.6 runs count as qualification evidence.
extern crate self as oteryn_game_server;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;

use durability::native_admission_source::{
    DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
    PendingPublication, SourceObservation,
};
use durability::{DurabilityError, DurabilityRoot};
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
    let migration = include_str!("../migrations/0003_native_admission_source.sql");
    for required in [
        "registration_id SMALLINT PRIMARY KEY CHECK (registration_id = 1)",
        "source_authority !~ '[^A-Za-z0-9._:/-]'",
        "octet_length(descriptor_facts) BETWEEN 1 AND 4096",
        "octet_length(operation_binding) BETWEEN 1 AND 16384",
        "slot_id SMALLINT NOT NULL CHECK (slot_id IN (1, 2))",
        "'ReadAccountSecurityV1'",
        "'ReadFreshSigningTrustV1'",
        "'ReadRecoveryAccountSecurityV2'",
        "'ReadRecoverySigningTrustV2'",
        "native source registration cannot roll back or be recreated",
        "native source floor cannot roll back or be rewritten",
        "native source canonical history is immutable",
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
                assert!(matches!(
                    root.pending_native_source_publications().await,
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
                root.initialize_native_admission_source(provenance(), descriptor(1, 1, 10))
                    .await?;
                root.register_native_admission_descriptor(descriptor(2, 2, 20))
                    .await?;
                root.register_native_admission_descriptor(descriptor(2, 2, 20))
                    .await?;
                assert!(matches!(
                    root.register_native_admission_descriptor(descriptor(2, 3, 20))
                        .await,
                    Err(DurabilityError::Unavailable)
                ));

                let fresh_10 = account_observation(
                    NativeSourceOperation::ReadAccountSecurityV1,
                    10,
                    "fresh:10",
                    100,
                    vec![1],
                );
                root.accept_native_source_observation(fresh_10.clone())
                    .await?;
                root.accept_native_source_observation(fresh_10.clone())
                    .await?;
                let recovery_11 = account_observation(
                    NativeSourceOperation::ReadRecoveryAccountSecurityV2,
                    11,
                    "recovery:11",
                    101,
                    vec![2],
                );
                root.accept_native_source_observation(recovery_11.clone())
                    .await?;
                assert!(matches!(
                    root.accept_native_source_observation(fresh_10).await,
                    Err(DurabilityError::Unavailable)
                ));
                let fresh_12 = account_observation(
                    NativeSourceOperation::ReadAccountSecurityV1,
                    12,
                    "fresh:12",
                    102,
                    vec![3],
                );
                root.accept_native_source_observation(fresh_12).await?;
                assert!(matches!(
                    root.accept_native_source_observation(recovery_11).await,
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
                    .accept_native_source_observation(maximum.clone())
                    .await?;
                restarted.accept_native_source_observation(maximum).await?;
                assert!(matches!(
                    restarted
                        .accept_native_source_observation(account_observation(
                            NativeSourceOperation::ReadAccountSecurityV1,
                            u64::MAX - 1,
                            "fresh:lower",
                            104,
                            vec![5]
                        ))
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
                root.initialize_native_admission_source(provenance(), descriptor(1, 1, 10))
                    .await?;

                let wrong = signing_observation(
                    NativeSourceOperation::ReadAccountSecurityV1,
                    FRESH_ISSUER,
                    FRESH_PROFILE,
                    "fresh_admission",
                    "kid-1",
                    1,
                );
                assert!(matches!(
                    root.accept_native_source_observation(wrong).await,
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
                    root.accept_native_source_observation(wrong_context).await,
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
                root.accept_native_source_observation(key_one).await?;
                let key_two_lower = signing_observation(
                    NativeSourceOperation::ReadFreshSigningTrustV1,
                    FRESH_ISSUER,
                    FRESH_PROFILE,
                    "fresh_admission",
                    "kid-2",
                    1,
                );
                assert!(matches!(
                    root.accept_native_source_observation(key_two_lower).await,
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
                root.accept_native_source_observation(key_two_newer.clone())
                    .await?;
                root.accept_native_source_observation(key_two_newer).await?;

                let recovery = signing_observation(
                    NativeSourceOperation::ReadRecoverySigningTrustV2,
                    RECOVERY_ISSUER,
                    RECOVERY_PROFILE,
                    "existing_actor_recovery",
                    "recovery-kid",
                    1,
                );
                root.accept_native_source_observation(recovery).await?;
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
                let authority = "a".repeat(128);
                root.initialize_native_admission_source(
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
                root.accept_native_source_observation(at_max).await?;

                let mut long_authority = account_observation(
                    NativeSourceOperation::ReadAccountSecurityV1,
                    2,
                    "authority-long",
                    101,
                    vec![1],
                );
                long_authority.source_authority = "b".repeat(129);
                assert!(matches!(
                    root.accept_native_source_observation(long_authority).await,
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
                    root.accept_native_source_observation(bad_authority).await,
                    Err(DurabilityError::Unavailable)
                ));
                assert!(matches!(
                    root.register_native_admission_descriptor(descriptor_with_facts(
                        2,
                        vec![2; 4097],
                        20
                    ))
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
                    root.accept_native_source_observation(long_decision).await,
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
                    root.accept_native_source_observation(long_body).await,
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
            root.initialize_native_admission_source(provenance(), descriptor(2, 2, 20)).await?;
            assert!(matches!(
                root.register_native_admission_descriptor(descriptor(1, 1, 10)).await,
                Err(DurabilityError::Unavailable)
            ));
            let base = account_observation(
                NativeSourceOperation::ReadAccountSecurityV1, 20, "fresh:20", 200, vec![1]);
            let advance = account_observation(
                NativeSourceOperation::ReadAccountSecurityV1, 21, "fresh:21", 201, vec![2]);
            root.accept_native_source_observation(base.clone()).await?;

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
                root.accept_native_source_observation(advance.clone()).await,
                Err(DurabilityError::Database(_))
            ));
            sqlx::query("DROP TRIGGER reject_native_source_floor_advance ON game_durability_native_source_floors")
                .execute(&mut fault).await?;
            sqlx::query("DROP FUNCTION reject_native_source_floor_advance()")
                .execute(&mut fault).await?;
            fault.close().await?;
            drop(root);

            let restarted = ready_root(&database_url).await?;
            restarted.accept_native_source_observation(base).await?;
            restarted.accept_native_source_observation(advance).await?;
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
                root.initialize_native_admission_source(provenance(), descriptor(1, 1, 10))
                    .await?;
                let first = vec![1; 16_384];
                let second = vec![2; 16_384];
                let third = vec![3];
                let first_slot = root
                    .checkpoint_native_source_publication(first.clone(), 300)
                    .await?;
                drop(root);

                let restarted = ready_root(&database_url).await?;
                assert_eq!(
                    restarted
                        .checkpoint_native_source_publication(first.clone(), 999)
                        .await?,
                    first_slot
                );
                let after_replay = restarted.pending_native_source_publications().await?;
                let replayed = pending(&after_replay, &first).ok_or("missing replayed slot")?;
                assert_eq!(replayed.checkpointed_at, 300);
                let second_slot = restarted
                    .checkpoint_native_source_publication(second.clone(), 301)
                    .await?;
                let aggregate: usize = restarted
                    .pending_native_source_publications()
                    .await?
                    .iter()
                    .map(|publication| publication.operation_binding.len())
                    .sum();
                assert_eq!(aggregate, 32_768);
                assert!(matches!(
                    restarted
                        .checkpoint_native_source_publication(third.clone(), 302)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                assert!(matches!(
                    restarted
                        .checkpoint_native_source_publication(vec![4; 16_385], 303)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                assert!(matches!(
                    restarted
                        .clear_native_source_publication(first_slot, third.clone())
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                restarted
                    .clear_native_source_publication(first_slot, first)
                    .await?;
                drop(restarted);

                let reconciled = ready_root(&database_url).await?;
                let remaining = reconciled.pending_native_source_publications().await?;
                assert_eq!(remaining.len(), 1);
                assert_eq!(remaining[0].slot_id, second_slot);
                assert_eq!(remaining[0].operation_binding, second);
                assert_eq!(
                    reconciled
                        .checkpoint_native_source_publication(third, 302)
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
