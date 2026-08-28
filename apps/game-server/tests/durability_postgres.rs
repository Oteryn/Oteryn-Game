#[path = "../src/durability/mod.rs"]
mod durability;
#[path = "support/postgres.rs"]
mod postgres;

use durability::{
    AdmissionReconnectJournal, DurabilityError, MigrationExecutor, SchemaCompatibility,
};
use oteryn_game_server::foundation::{
    AuthenticatedTransportRefV1, AuthorityEvidenceFenceV1, ChannelId, CharacterId, CommandId,
    ConnectionGeneration, ControlLossEpochRefV1, Fnd02ReconciliationFenceV1, GameSessionId,
    PendingCommandDispositionV1, PendingCommandReconciliationV1, ProtectionEntitlementV1,
    ReconnectAttemptRef, ReconnectAuthorityFenceV1, ReconnectCommitActionV1,
    ReconnectCommitCompletionV1, ReconnectCommitDispositionV1, ReconnectCompatibilityEvidenceV1,
    ReconnectConnectionFenceV1, ReconnectContinuityV1, ReconnectCurrentAuthorityV1,
    ReconnectDurabilityErrorV1, ReconnectDurabilityFlowV1, ReconnectDurabilityRecordV1,
    ReconnectIdentityV1, ReconnectPrepareActionV1, ReconnectPrepareCompletionV1,
    ReconnectPrepareDispositionV1, ReconnectProjectionDecisionV1, ReconnectProofV1,
    RuntimeScopeRefV1, ScopeOwnershipGeneration, StateDomainRevisionV1, WorldId,
};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type CrossEpochSessionRow = (
    i64,
    i64,
    i64,
    i64,
    Option<Vec<u8>>,
    i16,
    i16,
    Option<Vec<u8>>,
);

fn foundation_error(error: ReconnectDurabilityErrorV1) -> std::io::Error {
    std::io::Error::other(format!(
        "Foundation V1 record construction failed: {error:?}"
    ))
}

fn unix_now() -> Result<i64, ReconnectDurabilityErrorV1> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?
        .as_secs()
        .try_into()
        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)
}

async fn postgres_clock(pool: &sqlx::PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT FLOOR(EXTRACT(EPOCH FROM clock_timestamp()))::BIGINT")
        .fetch_one(pool)
        .await
}

fn uuid_v7(raw: u64) -> [u8; 16] {
    let mut value = [0u8; 16];
    value[8..].copy_from_slice(&raw.to_be_bytes());
    value[6] = 0x70;
    value[8] = (value[8] & 0x3f) | 0x80;
    value
}

fn record(
    game_session_raw: u64,
    attempt_raw: u64,
    transport_byte: u8,
    now: i64,
) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    record_with_prepared_deadline(
        game_session_raw,
        attempt_raw,
        transport_byte,
        now,
        now + 115,
    )
}

fn record_with_prepared_deadline(
    game_session_raw: u64,
    attempt_raw: u64,
    transport_byte: u8,
    now: i64,
    prepared_deadline: i64,
) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    record_for_epoch(
        game_session_raw,
        attempt_raw,
        transport_byte,
        now,
        prepared_deadline,
        3,
        7,
        8,
        0x55,
    )
}

#[allow(clippy::too_many_arguments)]
fn record_for_epoch(
    game_session_raw: u64,
    attempt_raw: u64,
    transport_byte: u8,
    now: i64,
    prepared_deadline: i64,
    control_loss_epoch: u64,
    predecessor_generation: u64,
    candidate_generation: u64,
    recovery_nonce_byte: u8,
) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    let game_session_id = GameSessionId::decode(&uuid_v7(game_session_raw))
        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let character_id = CharacterId::decode(&uuid_v7(11))
        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let world_id = WorldId::decode(&uuid_v7(12))
        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let channel_id = ChannelId::decode(&uuid_v7(13))
        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let identity = ReconnectIdentityV1::new(
        game_session_id,
        ReconnectAttemptRef::new(attempt_raw)
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
        "123e4567-e89b-12d3-a456-426614174000",
        character_id,
        world_id,
        RuntimeScopeRefV1::channel(world_id, channel_id),
    )?;
    let connection = ReconnectConnectionFenceV1::new(
        ConnectionGeneration::new(predecessor_generation)
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
        ConnectionGeneration::new(candidate_generation)
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
        AuthenticatedTransportRefV1::decode(&[transport_byte; 16])
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
    )?;
    let authority = ReconnectAuthorityFenceV1::new(
        9,
        ScopeOwnershipGeneration::new(10)
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
    )?;
    let continuity = ReconnectContinuityV1::new(
        ControlLossEpochRefV1::new(control_loss_epoch)?,
        now + 120,
        prepared_deadline,
        ProtectionEntitlementV1::unused(),
    )?;
    let fnd02 = Fnd02ReconciliationFenceV1::new(
        CommandId::new(3).map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
        vec![
            PendingCommandReconciliationV1::new(
                CommandId::new(1).map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
                PendingCommandDispositionV1::PendingOriginal,
            ),
            PendingCommandReconciliationV1::new(
                CommandId::new(2).map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
                PendingCommandDispositionV1::TerminalOutcomeRetained,
            ),
        ],
        41,
        vec![
            StateDomainRevisionV1::new(1, 4)?,
            StateDomainRevisionV1::new(2, 7)?,
        ],
    )?;
    let platform = AuthorityEvidenceFenceV1::new(
        "platform-security",
        "reconnect",
        "account",
        "sec:17",
        "decision:sec:17",
        now,
    )?;
    let trust = AuthorityEvidenceFenceV1::new(
        "proof-trust",
        "reconnect",
        "recovery-key",
        "trust:21",
        "decision:trust:21",
        now,
    )?;
    let compatibility = ReconnectCompatibilityEvidenceV1::new(
        1,
        1,
        "rules:1",
        "content:2",
        "map:3",
        "world:4",
        12,
        platform,
        trust,
        Some(now + 110),
    )?;
    ReconnectDurabilityRecordV1::new(
        identity,
        connection,
        authority,
        continuity,
        ReconnectProofV1::ReauthenticatedRecovery {
            recovery_grant_nonce: [recovery_nonce_byte; 32],
        },
        fnd02,
        compatibility,
    )
}

fn postgres_e2e_is_configured() -> Result<bool, Box<dyn std::error::Error>> {
    match postgres::postgres_e2e_availability()? {
        postgres::PostgresE2eAvailability::Configured => Ok(true),
        postgres::PostgresE2eAvailability::NotConfigured => {
            eprintln!(
                "PostgreSQL E2E NOT_APPLICABLE: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured; real database assertions run in the dedicated configured harness"
            );
            Ok(false)
        }
    }
}

#[test]
fn isolated_postgres_guard_classifies_absence_and_rejects_unsafe_configuration()
-> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        postgres::classify_e2e_admin_url(None)?,
        postgres::PostgresE2eAvailability::NotConfigured
    );
    assert_eq!(
        postgres::classify_e2e_admin_url(Some(
            "postgresql://oteryn_test_admin:secret@127.0.0.1:5432/postgres"
        ))?,
        postgres::PostgresE2eAvailability::Configured
    );
    assert!(matches!(
        postgres::classify_e2e_admin_url(Some(
            "postgresql://oteryn_test_admin:secret@remote.example/postgres"
        )),
        Err(postgres::IsolatedPostgresError::UnsafeAdminUrl)
    ));
    assert!(
        postgres::validate_admin_url(
            "postgresql://oteryn_test_admin:secret@remote.example/postgres"
        )
        .is_err()
    );
    assert!(
        postgres::validate_admin_url(
            "postgresql://oteryn_test_admin:secret@127.0.0.1:5432/postgres?host=/tmp"
        )
        .is_err()
    );
    assert!(postgres::validate_admin_url(
        "postgresql://oteryn_test_admin:secret@127.0.0.1:5432/postgres?options=-c%20search_path%3Dpublic"
    )
    .is_err());
    Ok(())
}

#[test]
fn fresh_migration_applies_only_the_embedded_game_ledger() -> Result<(), Box<dyn std::error::Error>>
{
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("fresh_migration").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                assert_eq!(executor.inspect().await?, SchemaCompatibility::Compatible);
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn same_prepare_replay_returns_the_existing_durable_disposition()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("same_attempt_replay").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let (_flow, request) = ReconnectDurabilityFlowV1::begin(
                    record(10, 1, 0x11, unix_now().map_err(foundation_error)?)
                        .map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&request).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                assert_eq!(
                    journal.prepare(&request).await?,
                    ReconnectPrepareDispositionV1::ExistingPrepared
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn same_prepare_replay_survives_process_replacement() -> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            if std::env::var_os("OTERYN_DURABILITY_REPLAY_CHILD").is_some() {
                let database_url = std::env::var("OTERYN_DURABILITY_REPLAY_DATABASE_URL")?;
                let record_now = std::env::var("OTERYN_DURABILITY_REPLAY_RECORD_NOW")?.parse()?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let (_flow, request) = ReconnectDurabilityFlowV1::begin(
                    record(11, 2, 0x22, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&request).await?,
                    ReconnectPrepareDispositionV1::ExistingPrepared
                );
                return Ok(());
            }

            let database = postgres::IsolatedPostgres::create("cross_process_replay").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (_flow, request) = ReconnectDurabilityFlowV1::begin(
                    record(11, 2, 0x22, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&request).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );

                let status = Command::new(std::env::current_exe()?)
                    .arg("--exact")
                    .arg("same_prepare_replay_survives_process_replacement")
                    .env("OTERYN_DURABILITY_REPLAY_CHILD", "1")
                    .env("OTERYN_DURABILITY_REPLAY_DATABASE_URL", &database_url)
                    .env(
                        "OTERYN_DURABILITY_REPLAY_RECORD_NOW",
                        record_now.to_string(),
                    )
                    .status()?;
                assert!(
                    status.success(),
                    "fresh process did not replay the durable disposition"
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn transport_ref_collision_is_durable_and_same_attempt_replays_terminal()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("transport_ref_collision").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (_first_flow, first) = ReconnectDurabilityFlowV1::begin(
                    record(20, 1, 0x33, record_now).map_err(foundation_error)?,
                );
                let (_colliding_flow, colliding) = ReconnectDurabilityFlowV1::begin(
                    record(21, 1, 0x33, record_now).map_err(foundation_error)?,
                );

                assert_eq!(
                    journal.prepare(&first).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                assert_eq!(
                    journal.prepare(&colliding).await?,
                    ReconnectPrepareDispositionV1::RejectedTransportRefCollision
                );
                assert_eq!(
                    journal.prepare(&colliding).await?,
                    ReconnectPrepareDispositionV1::ExistingTerminal
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn one_prepared_attempt_and_eight_attempt_epoch_limits_are_enforced_in_postgres()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("attempt_capacity").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (_first_flow, first) = ReconnectDurabilityFlowV1::begin(
                    record(30, 1, 0x41, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&first).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );

                for attempt in 2_u64..=8 {
                    let transport = u8::try_from(0x40_u64 + attempt)?;
                    let (_flow, request) = ReconnectDurabilityFlowV1::begin(
                        record(30, attempt, transport, record_now).map_err(foundation_error)?,
                    );
                    assert_eq!(
                        journal.prepare(&request).await?,
                        ReconnectPrepareDispositionV1::RejectedConcurrentPrepared
                    );
                }
                let (_ninth_flow, ninth) = ReconnectDurabilityFlowV1::begin(
                    record(30, 9, 0x49, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&ninth).await?,
                    ReconnectPrepareDispositionV1::AttemptCapacityExceeded
                );
                assert_eq!(
                    journal.prepare(&first).await?,
                    ReconnectPrepareDispositionV1::ExistingPrepared
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn same_attempt_with_changed_record_conflicts_without_consuming_the_new_ref()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("idempotency_conflict").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (_first_flow, first) = ReconnectDurabilityFlowV1::begin(
                    record(40, 1, 0x51, record_now).map_err(foundation_error)?,
                );
                let (_changed_flow, changed) = ReconnectDurabilityFlowV1::begin(
                    record(40, 1, 0x52, record_now).map_err(foundation_error)?,
                );
                let (_new_flow, new_attempt) = ReconnectDurabilityFlowV1::begin(
                    record(41, 1, 0x52, record_now).map_err(foundation_error)?,
                );

                assert_eq!(
                    journal.prepare(&first).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                assert_eq!(
                    journal.prepare(&changed).await?,
                    ReconnectPrepareDispositionV1::IdempotencyConflict
                );
                assert_eq!(
                    journal.prepare(&new_attempt).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn concurrent_same_attempt_reconciles_to_one_prepared_and_one_existing_prepared()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("same_attempt_race").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let (_flow, request) = ReconnectDurabilityFlowV1::begin(
                    record(50, 1, 0x61, unix_now().map_err(foundation_error)?)
                        .map_err(foundation_error)?,
                );
                let barrier = Arc::new(tokio::sync::Barrier::new(2));
                let first = {
                    let barrier = Arc::clone(&barrier);
                    let journal = journal.clone();
                    let request = request.clone();
                    tokio::spawn(async move {
                        barrier.wait().await;
                        journal.prepare(&request).await
                    })
                };
                let second = {
                    let barrier = Arc::clone(&barrier);
                    let journal = journal.clone();
                    let request = request.clone();
                    tokio::spawn(async move {
                        barrier.wait().await;
                        journal.prepare(&request).await
                    })
                };
                let mut dispositions = [first.await??, second.await??];
                dispositions.sort_unstable_by_key(|disposition| match disposition {
                    ReconnectPrepareDispositionV1::Prepared => 0,
                    ReconnectPrepareDispositionV1::ExistingPrepared => 1,
                    _ => 2,
                });
                assert_eq!(
                    dispositions,
                    [
                        ReconnectPrepareDispositionV1::Prepared,
                        ReconnectPrepareDispositionV1::ExistingPrepared,
                    ]
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn exact_prepared_attempt_commits_once_and_reconciles_after_response_loss()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("commit_reconcile").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (mut flow, prepare) = ReconnectDurabilityFlowV1::begin(
                    record(60, 1, 0x71, record_now).map_err(foundation_error)?,
                );

                assert_eq!(
                    journal.prepare(&prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                assert_eq!(
                    flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                        &prepare,
                        ReconnectPrepareDispositionV1::Prepared,
                    ))
                    .map_err(foundation_error)?,
                    ReconnectPrepareActionV1::AwaitFinalRevalidation
                );
                let current =
                    ReconnectCurrentAuthorityV1::from_record(prepare.record(), record_now)
                        .map_err(foundation_error)?;
                let commit = flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;

                assert_eq!(
                    journal.commit(&commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );
                assert_eq!(
                    journal.commit(&commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );
                drop(journal);
                let recovered_journal =
                    AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                assert_eq!(
                    flow.accept_commit_completion(ReconnectCommitCompletionV1::for_request(
                        &commit,
                        ReconnectCommitDispositionV1::Committed,
                    ))
                    .map_err(foundation_error)?,
                    ReconnectCommitActionV1::ReconcileSameAttempt
                );
                assert_eq!(
                    flow.accept_reconciliation(
                        recovered_journal.reconcile(&prepare).await?,
                        ScopeOwnershipGeneration::new(10)
                            .map_err(|_error| std::io::Error::other("invalid scope generation"))?,
                    )
                    .map_err(foundation_error)?,
                    ReconnectProjectionDecisionV1::InstallController {
                        generation: ConnectionGeneration::new(8).map_err(|_error| {
                            std::io::Error::other("invalid connection generation")
                        })?,
                        transport_ref: AuthenticatedTransportRefV1::decode(&[0x71; 16])
                            .map_err(|_error| std::io::Error::other("invalid transport ref"))?,
                    }
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn committed_session_accepts_a_later_non_reused_control_loss_epoch()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("cross_epoch_reconnect").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let first_now = unix_now().map_err(foundation_error)?;
                let (mut first_flow, first_prepare) = ReconnectDurabilityFlowV1::begin(
                    record(63, 1, 0xd1, first_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&first_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                first_flow
                    .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                        &first_prepare,
                        ReconnectPrepareDispositionV1::Prepared,
                    ))
                    .map_err(foundation_error)?;
                let first_current =
                    ReconnectCurrentAuthorityV1::from_record(first_prepare.record(), first_now)
                        .map_err(foundation_error)?;
                let first_commit = first_flow
                    .authorize_commit(first_current, first_now)
                    .map_err(foundation_error)?;
                assert_eq!(
                    journal.commit(&first_commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );

                let second_now = first_now + 1;
                let (mut second_flow, second_prepare) = ReconnectDurabilityFlowV1::begin(
                    record_for_epoch(
                        63,
                        2,
                        0xd2,
                        second_now,
                        second_now + 115,
                        4,
                        8,
                        9,
                        0x56,
                    )
                    .map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&second_prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared,
                    "a later authoritative loss epoch must replace the committed transport fence"
                );

                let (_changed_grace_flow, changed_grace_prepare) =
                    ReconnectDurabilityFlowV1::begin(
                        record_for_epoch(
                            63,
                            3,
                            0xd3,
                            second_now + 1,
                            second_now + 116,
                            4,
                            8,
                            9,
                            0x57,
                        )
                        .map_err(foundation_error)?,
                    );
                assert_eq!(
                    journal.prepare(&changed_grace_prepare).await?,
                    ReconnectPrepareDispositionV1::RejectedStaleAuthority,
                    "attempts in one loss epoch cannot restart or extend its original grace"
                );
                assert_eq!(
                    journal.prepare(&changed_grace_prepare).await?,
                    ReconnectPrepareDispositionV1::ExistingTerminal
                );

                second_flow
                    .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                        &second_prepare,
                        ReconnectPrepareDispositionV1::Prepared,
                    ))
                    .map_err(foundation_error)?;
                let second_current =
                    ReconnectCurrentAuthorityV1::from_record(second_prepare.record(), second_now)
                        .map_err(foundation_error)?;
                let second_commit = second_flow
                    .authorize_commit(second_current, second_now)
                    .map_err(foundation_error)?;
                assert_eq!(
                    journal.commit(&second_commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );
                assert_eq!(
                    journal.commit(&second_commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );
                assert!(matches!(
                    journal.commit(&first_commit).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                assert!(matches!(
                    journal.reconcile(&first_prepare).await,
                    Err(DurabilityError::InvalidStoredState)
                ));

                let (_reused_epoch_flow, reused_epoch_prepare) =
                    ReconnectDurabilityFlowV1::begin(
                        record_for_epoch(
                            63,
                            4,
                            0xd4,
                            second_now,
                            second_now + 115,
                            3,
                            9,
                            10,
                            0x58,
                        )
                        .map_err(foundation_error)?,
                    );
                assert_eq!(
                    journal.prepare(&reused_epoch_prepare).await?,
                    ReconnectPrepareDispositionV1::RejectedStaleAuthority,
                    "a previously retained loss epoch cannot be reused"
                );
                assert_eq!(
                    journal.prepare(&reused_epoch_prepare).await?,
                    ReconnectPrepareDispositionV1::ExistingTerminal
                );

                for attempt in 5_u64..=10 {
                    let transport = u8::try_from(0xd0_u64 + attempt)?;
                    let nonce = u8::try_from(0x50_u64 + attempt)?;
                    let (_old_epoch_flow, old_epoch_prepare) = ReconnectDurabilityFlowV1::begin(
                        record_for_epoch(
                            63,
                            attempt,
                            transport,
                            second_now,
                            second_now + 115,
                            3,
                            9,
                            10,
                            nonce,
                        )
                        .map_err(foundation_error)?,
                    );
                    assert_eq!(
                        journal.prepare(&old_epoch_prepare).await?,
                        ReconnectPrepareDispositionV1::RejectedStaleAuthority
                    );
                }
                let (_old_epoch_capacity_flow, old_epoch_capacity) =
                    ReconnectDurabilityFlowV1::begin(
                        record_for_epoch(
                            63,
                            11,
                            0xdb,
                            second_now,
                            second_now + 115,
                            3,
                            9,
                            10,
                            0x5b,
                        )
                        .map_err(foundation_error)?,
                    );
                assert_eq!(
                    journal.prepare(&old_epoch_capacity).await?,
                    ReconnectPrepareDispositionV1::AttemptCapacityExceeded,
                    "closed epochs retain the same eight-attempt hard bound"
                );

                let pool = sqlx::PgPool::connect(&database_url).await?;
                let session_id = second_prepare
                    .record()
                    .identity()
                    .game_session_id()
                    .as_bytes()
                    .to_vec();
                let session: CrossEpochSessionRow = sqlx::query_as(
                    "SELECT control_loss_epoch::BIGINT, original_grace_deadline, predecessor_generation::BIGINT, \
                            current_generation::BIGINT, current_transport_ref, session_state, \
                            attempt_count, prepared_attempt_ref \
                     FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(session_id.as_slice())
                .fetch_one(&pool)
                .await?;
                assert_eq!(session.0, 4);
                assert_eq!(session.1, second_now + 120);
                assert_eq!(session.2, 8);
                assert_eq!(session.3, 9);
                assert_eq!(session.4.as_deref(), Some([0xd2_u8; 16].as_slice()));
                assert_eq!(session.5, 2);
                assert_eq!(session.6, 2);
                assert!(session.7.is_none());

                let attempts_per_epoch: Vec<(i64, i64)> = sqlx::query_as(
                    "SELECT control_loss_epoch::BIGINT, COUNT(*) \
                     FROM game_durability_reconnect_attempts \
                     WHERE game_session_id = encode($1, 'hex')::uuid \
                     GROUP BY control_loss_epoch ORDER BY control_loss_epoch",
                )
                .bind(session_id.as_slice())
                .fetch_all(&pool)
                .await?;
                assert_eq!(attempts_per_epoch, vec![(3, 8), (4, 2)]);

                let grace_by_epoch: Vec<(i64, i64)> = sqlx::query_as(
                    "SELECT control_loss_epoch::BIGINT, \
                            (record_json::jsonb #>> '{continuity,original_grace_deadline}')::BIGINT \
                     FROM game_durability_reconnect_attempts \
                     WHERE game_session_id = encode($1, 'hex')::uuid \
                       AND reconnect_attempt_ref IN ($2, $3) \
                     ORDER BY control_loss_epoch",
                )
                .bind(session_id.as_slice())
                .bind(1_u64.to_be_bytes().as_slice())
                .bind(2_u64.to_be_bytes().as_slice())
                .fetch_all(&pool)
                .await?;
                assert_eq!(
                    grace_by_epoch,
                    vec![(3, first_now + 120), (4, second_now + 120)]
                );
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn stale_commit_terminalizes_the_prepared_attempt_for_reconciliation()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("stale_commit").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (mut flow, prepare) = ReconnectDurabilityFlowV1::begin(
                    record(61, 1, 0x72, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                assert_eq!(
                    flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                        &prepare,
                        ReconnectPrepareDispositionV1::Prepared,
                    ))
                    .map_err(foundation_error)?,
                    ReconnectPrepareActionV1::AwaitFinalRevalidation
                );
                let current =
                    ReconnectCurrentAuthorityV1::from_record(prepare.record(), record_now)
                        .map_err(foundation_error)?;
                let commit = flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;
                let pool = sqlx::PgPool::connect(&database_url).await?;
                sqlx::query(
                    "UPDATE game_durability_reconnect_sessions \
                     SET session_state = 2, current_generation = 8, current_transport_ref = $2 \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(
                    prepare
                        .record()
                        .identity()
                        .game_session_id()
                        .as_bytes()
                        .as_slice(),
                )
                .bind([0x99_u8; 16].as_slice())
                .execute(&pool)
                .await?;
                assert_eq!(
                    journal.commit(&commit).await?,
                    ReconnectCommitDispositionV1::RejectedStaleAuthority
                );
                assert_eq!(
                    flow.accept_commit_completion(ReconnectCommitCompletionV1::for_request(
                        &commit,
                        ReconnectCommitDispositionV1::RejectedStaleAuthority,
                    ))
                    .map_err(foundation_error)?,
                    ReconnectCommitActionV1::Terminal(
                        ReconnectCommitDispositionV1::RejectedStaleAuthority
                    )
                );
                assert_eq!(
                    journal.reconcile(&prepare).await?,
                    oteryn_game_server::foundation::ReconnectDurableReconciliationSnapshotV1::terminal(
                        prepare.record().clone(),
                    )
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn committed_replay_fails_closed_when_session_state_is_inconsistent()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("corrupt_commit_state").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let record_now = unix_now().map_err(foundation_error)?;
                let (mut flow, prepare) = ReconnectDurabilityFlowV1::begin(
                    record(62, 1, 0x73, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(foundation_error)?;
                let current =
                    ReconnectCurrentAuthorityV1::from_record(prepare.record(), record_now)
                        .map_err(foundation_error)?;
                let commit = flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;
                assert_eq!(
                    journal.commit(&commit).await?,
                    ReconnectCommitDispositionV1::Committed
                );
                let pool = sqlx::PgPool::connect(&database_url).await?;
                sqlx::query(
                    "UPDATE game_durability_reconnect_sessions SET session_state = 1 \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(
                    prepare
                        .record()
                        .identity()
                        .game_session_id()
                        .as_bytes()
                        .as_slice(),
                )
                .execute(&pool)
                .await?;
                assert!(matches!(
                    journal.commit(&commit).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn commit_row_lock_wait_cannot_outlive_authorization_deadline()
-> Result<(), Box<dyn std::error::Error>> {
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("commit_deadline_lock").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let lock_pool = sqlx::PgPool::connect(&database_url).await?;
                let record_now = postgres_clock(&lock_pool).await?;
                let (mut flow, prepare) = ReconnectDurabilityFlowV1::begin(
                    record(90, 1, 0xb1, record_now).map_err(foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&prepare).await?,
                    ReconnectPrepareDispositionV1::Prepared
                );
                flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(foundation_error)?;
                let current = ReconnectCurrentAuthorityV1::from_record(prepare.record(), record_now)
                    .map_err(foundation_error)?;
                let commit = flow
                    .authorize_commit(current, record_now)
                    .map_err(foundation_error)?;
                let deadline = commit.authorization().authorization_deadline();
                assert!(postgres_clock(&lock_pool).await? <= deadline);

                let session_id = prepare
                    .record()
                    .identity()
                    .game_session_id()
                    .as_bytes()
                    .to_vec();
                let mut lock = lock_pool.begin().await?;
                sqlx::query(
                    "SELECT game_session_id FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid FOR UPDATE",
                )
                .bind(session_id.as_slice())
                .fetch_one(&mut *lock)
                .await?;

                let blocked_journal = journal.clone();
                let blocked_commit = commit.clone();
                let blocked = tokio::spawn(async move {
                    blocked_journal.commit(&blocked_commit).await
                });
                tokio::time::sleep(Duration::from_millis(100)).await;
                assert!(
                    !blocked.is_finished(),
                    "commit must be waiting on the held per-session row lock"
                );
                while postgres_clock(&lock_pool).await? <= deadline {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                lock.commit().await?;

                assert_eq!(
                    blocked.await??,
                    ReconnectCommitDispositionV1::RejectedStaleAuthority
                );
                let session: (i64, Option<Vec<u8>>, i16, Option<Vec<u8>>) = sqlx::query_as(
                    "SELECT current_generation::BIGINT, current_transport_ref, session_state, prepared_attempt_ref \
                     FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(session_id.as_slice())
                .fetch_one(&lock_pool)
                .await?;
                assert_eq!(session.0, 7);
                assert!(session.1.is_none());
                assert_eq!(session.2, 1);
                assert!(session.3.is_none());
                let consumed: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM game_durability_recovery_grant_consumptions",
                )
                .fetch_one(&lock_pool)
                .await?;
                assert_eq!(consumed, 0, "expired commit must not consume recovery nonce");
                assert_eq!(
                    journal.reconcile(&prepare).await?,
                    oteryn_game_server::foundation::ReconnectDurableReconciliationSnapshotV1::terminal(
                        prepare.record().clone(),
                    )
                );
                lock_pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}

#[test]
fn prepare_row_lock_wait_cannot_outlive_prepared_deadline() -> Result<(), Box<dyn std::error::Error>>
{
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("prepare_deadline_lock").await?;
            let result = async {
                let database_url = database.database_url()?;
                let executor = MigrationExecutor::connect_migration(&database_url).await?;
                executor.apply_embedded_ledger().await?;
                let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
                let lock_pool = sqlx::PgPool::connect(&database_url).await?;
                let record_now = postgres_clock(&lock_pool).await?;
                let prepared_deadline = record_now + 2;
                let (_flow, request) = ReconnectDurabilityFlowV1::begin(
                    record_with_prepared_deadline(91, 1, 0xc1, record_now, prepared_deadline)
                        .map_err(foundation_error)?,
                );
                let session_id = request
                    .record()
                    .identity()
                    .game_session_id()
                    .as_bytes()
                    .to_vec();
                sqlx::query(
                    "INSERT INTO game_durability_reconnect_sessions (\
                        game_session_id, account_id, character_id, world_id, runtime_scope_kind, \
                        runtime_scope_world_id, runtime_scope_channel_id, runtime_scope_instance_id, \
                        control_loss_epoch, original_grace_deadline, predecessor_generation, \
                        character_lease_generation, scope_ownership_generation, current_generation\
                     ) VALUES (encode($1, 'hex')::uuid, '123e4567-e89b-12d3-a456-426614174000'::uuid, \
                        encode($2, 'hex')::uuid, encode($3, 'hex')::uuid, 1, encode($3, 'hex')::uuid, \
                        encode($4, 'hex')::uuid, NULL, 3, $5, 7, 9, 10, 7)",
                )
                .bind(session_id.as_slice())
                .bind(uuid_v7(11).as_slice())
                .bind(uuid_v7(12).as_slice())
                .bind(uuid_v7(13).as_slice())
                .bind(record_now + 120)
                .execute(&lock_pool)
                .await?;
                assert!(postgres_clock(&lock_pool).await? <= prepared_deadline);

                let mut lock = lock_pool.begin().await?;
                sqlx::query(
                    "SELECT game_session_id FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid FOR UPDATE",
                )
                .bind(session_id.as_slice())
                .fetch_one(&mut *lock)
                .await?;

                let blocked_journal = journal.clone();
                let blocked_request = request.clone();
                let blocked = tokio::spawn(async move {
                    blocked_journal.prepare(&blocked_request).await
                });
                tokio::time::sleep(Duration::from_millis(100)).await;
                assert!(
                    !blocked.is_finished(),
                    "prepare must be waiting on the held per-session row lock"
                );
                while postgres_clock(&lock_pool).await? <= prepared_deadline {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                lock.commit().await?;

                assert_eq!(
                    blocked.await??,
                    ReconnectPrepareDispositionV1::RejectedStaleAuthority
                );
                assert_eq!(
                    journal.prepare(&request).await?,
                    ReconnectPrepareDispositionV1::ExistingTerminal
                );
                let session: (i64, Option<Vec<u8>>, i16, i16, Option<Vec<u8>>) = sqlx::query_as(
                    "SELECT current_generation::BIGINT, current_transport_ref, session_state, attempt_count, prepared_attempt_ref \
                     FROM game_durability_reconnect_sessions \
                     WHERE game_session_id = encode($1, 'hex')::uuid",
                )
                .bind(session_id.as_slice())
                .fetch_one(&lock_pool)
                .await?;
                assert_eq!(session.0, 7);
                assert!(session.1.is_none());
                assert_eq!(session.2, 1);
                assert_eq!(session.3, 1);
                assert!(session.4.is_none());
                lock_pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}
