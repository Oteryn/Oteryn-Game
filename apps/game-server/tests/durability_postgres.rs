#[path = "../src/durability/mod.rs"]
mod durability;
#[path = "support/postgres.rs"]
mod postgres;

use durability::{AdmissionReconnectJournal, MigrationExecutor, SchemaCompatibility};
use oteryn_game_server::foundation::{
    AuthenticatedTransportRefV1, AuthorityEvidenceFenceV1, ChannelId, CharacterId, CommandId,
    ConnectionGeneration, ControlLossEpochRefV1, Fnd02ReconciliationFenceV1, GameSessionId,
    PendingCommandDispositionV1, PendingCommandReconciliationV1, ProtectionEntitlementV1,
    ReconnectAttemptRef, ReconnectAuthorityFenceV1, ReconnectCompatibilityEvidenceV1,
    ReconnectConnectionFenceV1, ReconnectContinuityV1, ReconnectDurabilityErrorV1,
    ReconnectDurabilityFlowV1, ReconnectDurabilityRecordV1, ReconnectIdentityV1,
    ReconnectPrepareDispositionV1, ReconnectProofV1, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    StateDomainRevisionV1, WorldId,
};
use std::process::Command;

fn uuid_v7(raw: u64) -> [u8; 16] {
    let mut value = [0u8; 16];
    value[8..].copy_from_slice(&raw.to_be_bytes());
    value[6] = 0x70;
    value[8] = (value[8] & 0x3f) | 0x80;
    value
}

fn record(
    attempt_raw: u64,
    transport_byte: u8,
) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    let game_session_id = GameSessionId::decode(&uuid_v7(10))
        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let character_id = CharacterId::decode(&uuid_v7(11))
        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let world_id = WorldId::decode(&uuid_v7(12))
        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let channel_id = ChannelId::decode(&uuid_v7(13))
        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let identity = ReconnectIdentityV1::new(
        game_session_id,
        ReconnectAttemptRef::new(attempt_raw)?,
        "123e4567-e89b-12d3-a456-426614174000",
        character_id,
        world_id,
        RuntimeScopeRefV1::channel(world_id, channel_id),
    )?;
    let connection = ReconnectConnectionFenceV1::new(
        ConnectionGeneration::new(7).map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
        ConnectionGeneration::new(8).map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
        AuthenticatedTransportRefV1::decode(&[transport_byte; 16])
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
    )?;
    let authority = ReconnectAuthorityFenceV1::new(9, ScopeOwnershipGeneration::new(10)?)?;
    let continuity = ReconnectContinuityV1::new(
        ControlLossEpochRefV1::new(3)?,
        120,
        115,
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
        100,
    )?;
    let trust = AuthorityEvidenceFenceV1::new(
        "proof-trust",
        "reconnect",
        "recovery-key",
        "trust:21",
        "decision:trust:21",
        101,
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
        Some(110),
    )?;
    ReconnectDurabilityRecordV1::new(
        identity,
        connection,
        authority,
        continuity,
        ReconnectProofV1::ReauthenticatedRecovery {
            recovery_grant_nonce: [0x55; 32],
        },
        fnd02,
        compatibility,
    )
}

#[test]
fn isolated_postgres_guard_rejects_nonlocal_socket_query_and_inherited_configuration() {
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
}

#[tokio::test]
async fn fresh_migration_applies_only_the_embedded_game_ledger()
-> Result<(), Box<dyn std::error::Error>> {
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
}

#[tokio::test]
async fn same_prepare_replay_returns_the_existing_durable_disposition()
-> Result<(), Box<dyn std::error::Error>> {
    let database = postgres::IsolatedPostgres::create("same_attempt_replay").await?;
    let result = async {
        let database_url = database.database_url()?;
        let executor = MigrationExecutor::connect_migration(&database_url).await?;
        executor.apply_embedded_ledger().await?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        let (_flow, request) = ReconnectDurabilityFlowV1::begin(record(1, 0x11)?);

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
}

#[tokio::test]
async fn same_prepare_replay_survives_process_replacement() -> Result<(), Box<dyn std::error::Error>>
{
    if std::env::var_os("OTERYN_DURABILITY_REPLAY_CHILD").is_some() {
        let database_url = std::env::var("OTERYN_DURABILITY_REPLAY_DATABASE_URL")?;
        let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
        let (_flow, request) = ReconnectDurabilityFlowV1::begin(record(2, 0x22)?);
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
        let (_flow, request) = ReconnectDurabilityFlowV1::begin(record(2, 0x22)?);
        assert_eq!(
            journal.prepare(&request).await?,
            ReconnectPrepareDispositionV1::Prepared
        );

        let status = Command::new(std::env::current_exe()?)
            .arg("--exact")
            .arg("same_prepare_replay_survives_process_replacement")
            .env("OTERYN_DURABILITY_REPLAY_CHILD", "1")
            .env("OTERYN_DURABILITY_REPLAY_DATABASE_URL", &database_url)
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
}
