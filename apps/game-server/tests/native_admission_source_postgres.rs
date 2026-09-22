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
    DescriptorRegistration, FreshStoreProvenance, PendingPublication, SourceObservation,
};
use durability::{DurabilityError, DurabilityRoot};
use sqlx::{Connection, Executor};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

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

fn provenance() -> FreshStoreProvenance {
    FreshStoreProvenance {
        namespace: "store:one".into(),
        authorization: "owner:approved".into(),
        initialized_at: 10,
    }
}

fn descriptor(revision: u64, facts: u8, installed_at: i64) -> DescriptorRegistration {
    DescriptorRegistration {
        revision,
        facts: vec![facts],
        installed_at,
    }
}

fn observation(
    operation: &str,
    source_revision: u64,
    decision_identity: &str,
    observed_at: i64,
    semantic_facts: u8,
) -> SourceObservation {
    SourceObservation {
        source_authority: "platform".into(),
        operation: operation.into(),
        semantic_namespace: "account:00000000-0000-4000-8000-000000000001".into(),
        source_revision,
        decision_identity: decision_identity.into(),
        observed_at,
        semantic_facts: vec![semantic_facts],
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
fn migration_declares_nonrollback_registration_floors_and_two_slots() {
    let migration = include_str!("../migrations/0003_native_admission_source.sql");
    assert!(migration.contains("registration_id SMALLINT PRIMARY KEY CHECK (registration_id = 1)"));
    assert!(migration.contains("slot_id SMALLINT NOT NULL CHECK (slot_id IN (1, 2))"));
    assert!(migration.contains("native source registration cannot roll back or be recreated"));
    assert!(migration.contains("native source floor cannot roll back or be rewritten"));
    assert!(migration.contains("native source canonical history is immutable"));
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

                let fresh = observation("ReadAccountSecurityV1", 10, "decision:fresh:10", 100, 1);
                root.accept_native_source_observation(fresh.clone()).await?;
                root.accept_native_source_observation(fresh.clone()).await?;
                let changed_equal =
                    observation("ReadAccountSecurityV1", 10, "decision:fresh:10", 101, 1);
                assert!(matches!(
                    root.accept_native_source_observation(changed_equal).await,
                    Err(DurabilityError::Unavailable)
                ));

                let recovery = observation(
                    "ReadRecoveryAccountSecurityV2",
                    11,
                    "decision:recovery:11",
                    102,
                    0,
                );
                root.accept_native_source_observation(recovery).await?;
                drop(root);

                let restarted = ready_root(&database_url).await?;
                assert!(matches!(
                    restarted
                        .register_native_admission_descriptor(descriptor(1, 1, 10))
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                restarted
                    .register_native_admission_descriptor(descriptor(2, 2, 20))
                    .await?;
                assert!(matches!(
                    restarted.accept_native_source_observation(fresh).await,
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
fn arbitrary_alternate_namespace_cannot_create_another_floor()
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
                create_database("native_source_namespace").await?;
            let result = async {
                migrate_postgres_17_6(&database_url).await?;
                let root = ready_root(&database_url).await?;
                root.initialize_native_admission_source(provenance(), descriptor(1, 1, 10))
                    .await?;
                root.accept_native_source_observation(observation(
                    "ReadAccountSecurityV1",
                    50,
                    "decision:50",
                    500,
                    1,
                ))
                .await?;

                let mut alternate =
                    observation("ReadAccountSecurityV1", 1, "decision:alternate", 501, 1);
                alternate.semantic_namespace =
                    "account:00000000-0000-7000-8000-000000000002".into();
                assert!(matches!(
                    root.accept_native_source_observation(alternate).await,
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
fn unknown_operation_rejects_before_retention() -> Result<(), Box<dyn std::error::Error>> {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (admin_url, database_name, database_url) =
                create_database("native_source_unknown_operation").await?;
            let result = async {
                migrate_postgres_17_6(&database_url).await?;
                let root = ready_root(&database_url).await?;
                root.initialize_native_admission_source(provenance(), descriptor(1, 1, 10))
                    .await?;
                assert!(matches!(
                    root.accept_native_source_observation(observation(
                        "UnknownOperation",
                        1,
                        "decision:unknown",
                        100,
                        1,
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
fn source_authority_max_plus_one_rejects_before_retention() -> Result<(), Box<dyn std::error::Error>>
{
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (admin_url, database_name, database_url) =
                create_database("native_source_authority_bound").await?;
            let result = async {
                migrate_postgres_17_6(&database_url).await?;
                let root = ready_root(&database_url).await?;
                root.initialize_native_admission_source(provenance(), descriptor(1, 1, 10))
                    .await?;

                let mut at_max =
                    observation("ReadAccountSecurityV1", 1, "decision:authority:max", 100, 1);
                at_max.source_authority = "a".repeat(128);
                root.accept_native_source_observation(at_max).await?;

                let mut above_max = observation(
                    "ReadAccountSecurityV1",
                    1,
                    "decision:authority:max-plus-one",
                    101,
                    1,
                );
                above_max.source_authority = "b".repeat(129);
                assert!(matches!(
                    root.accept_native_source_observation(above_max).await,
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
fn descriptor_max_plus_one_rejects_before_retention() -> Result<(), Box<dyn std::error::Error>> {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (admin_url, database_name, database_url) =
                create_database("native_source_descriptor_bound").await?;
            let result = async {
                migrate_postgres_17_6(&database_url).await?;
                let root = ready_root(&database_url).await?;
                root.initialize_native_admission_source(
                    provenance(),
                    DescriptorRegistration {
                        revision: 1,
                        facts: vec![1; 4096],
                        installed_at: 10,
                    },
                )
                .await?;
                assert!(matches!(
                    root.register_native_admission_descriptor(DescriptorRegistration {
                        revision: 2,
                        facts: vec![2; 4097],
                        installed_at: 20,
                    })
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
fn pending_checkpoint_max_plus_one_rejects_before_retention()
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
                create_database("native_source_pending_bound").await?;
            let result = async {
                migrate_postgres_17_6(&database_url).await?;
                let root = ready_root(&database_url).await?;
                root.initialize_native_admission_source(provenance(), descriptor(1, 1, 10))
                    .await?;
                root.checkpoint_native_source_publication(vec![1; 16384], 100)
                    .await?;
                assert!(matches!(
                    root.checkpoint_native_source_publication(vec![2; 16385], 101)
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
fn postgres_api_rolls_back_failed_floor_advance() -> Result<(), Box<dyn std::error::Error>> {
    if !configured() {
        skipped();
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (admin_url, database_name, database_url) =
                create_database("native_source_rollback").await?;
            let result = async {
                migrate_postgres_17_6(&database_url).await?;
                let root = ready_root(&database_url).await?;
                root.initialize_native_admission_source(provenance(), descriptor(1, 1, 10))
                    .await?;

                let base = observation("ReadAccountSecurityV1", 20, "decision:fresh:20", 200, 1);
                let advance = observation("ReadAccountSecurityV1", 21, "decision:fresh:21", 201, 0);
                root.accept_native_source_observation(base.clone()).await?;

                let mut fault = sqlx::PgConnection::connect(&database_url).await?;
                sqlx::query(
                    "CREATE FUNCTION reject_native_source_floor_advance() RETURNS trigger \
                     LANGUAGE plpgsql AS $$ BEGIN \
                     IF NEW.source_revision = 21 THEN \
                         RAISE EXCEPTION 'forced native source rollback' USING ERRCODE = '23514'; \
                     END IF; RETURN NEW; END; $$",
                )
                .execute(&mut fault)
                .await?;
                sqlx::query(
                    "CREATE TRIGGER reject_native_source_floor_advance \
                     BEFORE UPDATE ON game_durability_native_source_floors \
                     FOR EACH ROW EXECUTE FUNCTION reject_native_source_floor_advance()",
                )
                .execute(&mut fault)
                .await?;

                assert!(matches!(
                    root.accept_native_source_observation(advance.clone()).await,
                    Err(DurabilityError::Database(_))
                ));
                sqlx::query(
                    "DROP TRIGGER reject_native_source_floor_advance \
                     ON game_durability_native_source_floors",
                )
                .execute(&mut fault)
                .await?;
                sqlx::query("DROP FUNCTION reject_native_source_floor_advance()")
                    .execute(&mut fault)
                    .await?;
                fault.close().await?;
                drop(root);

                let restarted = ready_root(&database_url).await?;
                restarted.accept_native_source_observation(base).await?;
                restarted.accept_native_source_observation(advance).await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            cleanup_database(&admin_url, &database_name).await?;
            result
        })
}

#[test]
fn postgres_api_reconciles_lost_response_and_fixed_slots_after_restart()
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

                let first = vec![1];
                let second = vec![2];
                let third = vec![3];
                let first_slot = root
                    .checkpoint_native_source_publication(first.clone(), 300)
                    .await?;
                drop(root);

                let restarted = ready_root(&database_url).await?;
                let replayed_slot = restarted
                    .checkpoint_native_source_publication(first.clone(), 999)
                    .await?;
                assert_eq!(replayed_slot, first_slot);
                let after_replay = restarted.pending_native_source_publications().await?;
                let replayed = pending(&after_replay, &first).ok_or("missing replayed slot")?;
                assert_eq!(replayed.slot_id, first_slot);
                assert_eq!(replayed.checkpointed_at, 300);

                let second_slot = restarted
                    .checkpoint_native_source_publication(second.clone(), 301)
                    .await?;
                assert_ne!(second_slot, first_slot);
                assert!(matches!(
                    restarted
                        .checkpoint_native_source_publication(third.clone(), 302)
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                assert!(matches!(
                    restarted
                        .clear_native_source_publication(first_slot, third.clone())
                        .await,
                    Err(DurabilityError::Unavailable)
                ));
                assert_eq!(
                    restarted.pending_native_source_publications().await?.len(),
                    2
                );

                restarted
                    .clear_native_source_publication(first_slot, first)
                    .await?;
                drop(restarted);

                let reconciled = ready_root(&database_url).await?;
                let pending_after_clear = reconciled.pending_native_source_publications().await?;
                assert_eq!(pending_after_clear.len(), 1);
                assert_eq!(pending_after_clear[0].slot_id, second_slot);
                assert_eq!(pending_after_clear[0].operation_binding, second);

                let replacement_slot = reconciled
                    .checkpoint_native_source_publication(third, 302)
                    .await?;
                assert_eq!(replacement_slot, first_slot);
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            cleanup_database(&admin_url, &database_name).await?;
            result
        })
}
