use crate::durability::{AdmissionReconnectJournalV2, MigrationExecutor};
use crate::foundation::{
    AccountPresenceClaimV1, AuthenticatedTransportRefV1, AuthorityEvidenceFenceV1, ChannelId,
    CharacterId, CharacterLease, CharacterWorldEligibilityClaimV1, CommandId, ConnectionGeneration,
    ControlLossEpochRefV1, Fnd02ReconciliationFenceV1, FreshAdmissionCommit, FreshAdmissionFacts,
    GameSessionAuthoritySnapshot, GameSessionId, GameSessionState, ProtectionEntitlementV1,
    ReconnectAttemptBudgetV1, ReconnectAttemptRef, ReconnectAuthorityFenceV1,
    ReconnectCandidateBindingV1, ReconnectCommitDispositionV1, ReconnectCompatibilityEvidenceV1,
    ReconnectConnectionFenceV1, ReconnectContinuityV1, ReconnectCurrentAuthorityV1,
    ReconnectDurabilityErrorV1, ReconnectDurabilityFlowV1, ReconnectDurabilityFlowV2,
    ReconnectDurabilityRecordV1, ReconnectDurableOutcomeV2,
    ReconnectDurableReconciliationSnapshotV1, ReconnectDurableReconciliationSnapshotV2,
    ReconnectDurableTerminalDispositionV1, ReconnectIdentityV1, ReconnectPrepareCompletionV1,
    ReconnectPrepareCompletionV2, ReconnectPrepareDispositionV1, ReconnectPrepareDispositionV2,
    ReconnectProjectionDecisionV2, ReconnectProofV1, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    TerminalGameSessionReplacementAuthorizationV1, WorldId,
};
use sqlx::{Connection, Executor, PgConnection};
use std::error::Error;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const ACCOUNT: &str = "123e4567-e89b-12d3-a456-426614174000";
static DB_SEQUENCE: AtomicU64 = AtomicU64::new(0);
type TestResult = Result<(), Box<dyn Error>>;

fn exact_current_authority(
    record: &ReconnectDurabilityRecordV1,
    observed_at: i64,
) -> Result<ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1> {
    ReconnectCurrentAuthorityV1::from_current_facts(
        record,
        Some(AccountPresenceClaimV1::new(
            record.identity().account_id(),
            record.identity().character_id(),
        )?),
        Some(CharacterWorldEligibilityClaimV1::new(
            record.identity().character_id(),
            record.identity().world_id(),
        )),
        Some(ReconnectCandidateBindingV1::new(
            record.identity().game_session_id(),
            record.identity().reconnect_attempt_ref(),
            record.connection().candidate(),
            record.connection().transport_ref(),
            record.continuity().prepared_deadline(),
        )?),
        record.identity().runtime_scope(),
        record.connection().predecessor(),
        record.authority(),
        record.continuity().control_loss_epoch(),
        record.continuity().original_grace_deadline(),
        record.proof().clone(),
        record.fnd02().clone(),
        record.compatibility().clone(),
        GameSessionState::Reconnectable,
        false,
        observed_at,
    )
}

struct IsolatedDatabase {
    admin_url: String,
    database_name: String,
}

impl IsolatedDatabase {
    async fn create(test_name: &str) -> Result<Self, Box<dyn Error>> {
        let admin_url = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
        if !(admin_url.starts_with("postgresql://oteryn_test_admin:")
            && (admin_url.contains("@127.0.0.1:5432/") || admin_url.contains("@localhost:5432/"))
            && admin_url.ends_with("/postgres")
            && !admin_url.contains(['?', '#', '\n', '\r']))
        {
            return Err("unsafe PostgreSQL test-admin URL".into());
        }
        let ordinal = DB_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let normalized: String = test_name
            .bytes()
            .map(|byte| {
                if byte.is_ascii_alphanumeric() {
                    char::from(byte.to_ascii_lowercase())
                } else {
                    '_'
                }
            })
            .collect();
        let database_name = format!(
            "oteryn_game_review_{}_{}_{}",
            std::process::id(),
            ordinal,
            normalized
        );
        let mut admin = PgConnection::connect(&admin_url).await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE DATABASE {database_name}"
            ))))
            .await?;
        Ok(Self {
            admin_url,
            database_name,
        })
    }

    fn database_url(&self) -> Result<String, Box<dyn Error>> {
        let prefix = self
            .admin_url
            .strip_suffix("/postgres")
            .ok_or("invalid PostgreSQL test-admin URL")?;
        Ok(format!("{prefix}/{}", self.database_name))
    }

    async fn cleanup(self) -> Result<(), Box<dyn Error>> {
        let mut admin = PgConnection::connect(&self.admin_url).await?;
        sqlx::query(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
                 WHERE datname = $1 AND pid <> pg_backend_pid()",
        )
        .bind(&self.database_name)
        .execute(&mut admin)
        .await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE IF EXISTS {}",
                self.database_name
            ))))
            .await?;
        Ok(())
    }
}

fn run_postgres_test<F>(future: F) -> TestResult
where
    F: Future<Output = TestResult>,
{
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(future)
}

async fn migrated_database(test_name: &str) -> Result<(IsolatedDatabase, String), Box<dyn Error>> {
    let database = IsolatedDatabase::create(test_name).await?;
    let database_url = database.database_url()?;
    let executor = MigrationExecutor::connect_migration(&database_url).await?;
    executor.apply_embedded_ledger().await?;
    Ok((database, database_url))
}

fn unix_now() -> Result<i64, Box<dyn Error>> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        .try_into()?)
}

fn uuid_v7(raw: u64) -> [u8; 16] {
    let mut value = [0_u8; 16];
    value[8..].copy_from_slice(&raw.to_be_bytes());
    value[6] = 0x70;
    value[8] = (value[8] & 0x3f) | 0x80;
    value
}

fn game_session(raw: u64) -> Result<GameSessionId, ReconnectDurabilityErrorV1> {
    GameSessionId::decode(&uuid_v7(raw)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
}

fn character(raw: u64) -> Result<CharacterId, ReconnectDurabilityErrorV1> {
    CharacterId::decode(&uuid_v7(raw)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
}

fn world(raw: u64) -> Result<WorldId, ReconnectDurabilityErrorV1> {
    WorldId::decode(&uuid_v7(raw)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
}

fn channel(raw: u64) -> Result<ChannelId, ReconnectDurabilityErrorV1> {
    ChannelId::decode(&uuid_v7(raw)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
}

fn candidate_record() -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    candidate_record_for_channel(13)
}

fn candidate_record_for_channel(
    channel_raw: u64,
) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    let world_id = world(12)?;
    let identity = ReconnectIdentityV1::new(
        game_session(20)?,
        ReconnectAttemptRef::new(1).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        ACCOUNT,
        character(11)?,
        world_id,
        RuntimeScopeRefV1::channel(world_id, channel(channel_raw)?),
    )?;
    let connection = ReconnectConnectionFenceV1::new(
        ConnectionGeneration::new(7).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        ConnectionGeneration::new(8).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        AuthenticatedTransportRefV1::decode(&[0x71; 16])
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
    )?;
    let authority = ReconnectAuthorityFenceV1::new(
        9,
        ScopeOwnershipGeneration::new(10).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
    )?;
    let continuity = ReconnectContinuityV1::new(
        ControlLossEpochRefV1::new(3)?,
        120,
        115,
        ProtectionEntitlementV1::unused(),
    )?;
    let fnd02 = Fnd02ReconciliationFenceV1::new(
        CommandId::new(3).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        vec![],
        41,
        vec![],
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

#[allow(clippy::too_many_arguments)]
fn postgres_record(
    now: i64,
    session_raw: u64,
    attempt_raw: u64,
    transport_byte: u8,
    predecessor_generation: u64,
    protection_entitlement: ProtectionEntitlementV1,
) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
    let world_id = world(12)?;
    let identity = ReconnectIdentityV1::new(
        game_session(session_raw)?,
        ReconnectAttemptRef::new(attempt_raw)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        ACCOUNT,
        character(11)?,
        world_id,
        RuntimeScopeRefV1::channel(world_id, channel(13)?),
    )?;
    let connection = ReconnectConnectionFenceV1::new(
        ConnectionGeneration::new(predecessor_generation)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        ConnectionGeneration::new(predecessor_generation + 1)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        AuthenticatedTransportRefV1::decode(&[transport_byte; 16])
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
    )?;
    let authority = ReconnectAuthorityFenceV1::new(
        9,
        ScopeOwnershipGeneration::new(10).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
    )?;
    let continuity = ReconnectContinuityV1::new(
        ControlLossEpochRefV1::new(3)?,
        now + 120,
        now + 115,
        protection_entitlement,
    )?;
    let fnd02 = Fnd02ReconciliationFenceV1::new(
        CommandId::new(3).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        vec![],
        41,
        vec![],
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
            recovery_grant_nonce: [transport_byte; 32],
        },
        fnd02,
        compatibility,
    )
}

fn replacement_authorization(
    candidate: &ReconnectDurabilityRecordV1,
    predecessor_raw: u64,
    current_connection_generation: u64,
) -> Result<TerminalGameSessionReplacementAuthorizationV1, ReconnectDurabilityErrorV1> {
    let facts =
        FreshAdmissionFacts::new([0x44; 32], character(11)?, world(12)?, channel(13)?, 9, 10)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let initial_transport = AuthenticatedTransportRefV1::decode(&[0x70; 16])
        .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let predecessor = game_session(predecessor_raw)?;
    let commit = FreshAdmissionCommit::from_facts(predecessor, facts, initial_transport)
        .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let snapshot = GameSessionAuthoritySnapshot::from_current_facts(
        commit,
        GameSessionState::Terminal,
        ConnectionGeneration::new(current_connection_generation)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        None,
        CharacterLease::new(character(11)?, 9)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        Some(CharacterWorldEligibilityClaimV1::new(
            character(11)?,
            world(12)?,
        )),
        RuntimeScopeRefV1::channel(world(12)?, channel(13)?),
        ScopeOwnershipGeneration::new(10).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
    )?
    .with_control_loss_continuity(
        candidate.continuity().control_loss_epoch(),
        candidate.continuity().original_grace_deadline(),
    )?;
    TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
        ACCOUNT,
        Some(&AccountPresenceClaimV1::new(ACCOUNT, character(11)?)?),
        predecessor,
        candidate.identity().game_session_id(),
        snapshot,
        candidate,
    )
}

async fn seed_current_actor_anchor(
    database_url: &str,
    session_raw: u64,
    now: i64,
) -> Result<(), Box<dyn Error>> {
    let mut connection = PgConnection::connect(database_url).await?;
    sqlx::query(
        "INSERT INTO game_durability_reconnect_sessions (\
                game_session_id, account_id, character_id, world_id, runtime_scope_kind, \
                runtime_scope_world_id, runtime_scope_channel_id, runtime_scope_instance_id, \
                control_loss_epoch, original_grace_deadline, predecessor_generation, \
                character_lease_generation, scope_ownership_generation, current_generation, \
                session_state\
             ) VALUES (\
                encode($1, 'hex')::uuid, $2::text::uuid, encode($3, 'hex')::uuid, \
                encode($4, 'hex')::uuid, 1, encode($4, 'hex')::uuid, \
                encode($5, 'hex')::uuid, NULL, 3, $6, 7, 9, 10, 7, 1\
             )",
    )
    .bind(uuid_v7(session_raw).as_slice())
    .bind(ACCOUNT)
    .bind(uuid_v7(11).as_slice())
    .bind(uuid_v7(12).as_slice())
    .bind(uuid_v7(13).as_slice())
    .bind(now + 120)
    .execute(&mut connection)
    .await?;
    sqlx::query(
        "INSERT INTO game_durability_control_loss_continuity (\
                character_id, control_loss_epoch, account_id, world_id, context_game_session_id, \
                original_grace_deadline, protection_entitlement_state, protection_rearm_state\
             ) VALUES (encode($1, 'hex')::uuid, 3, $2::text::uuid, encode($3, 'hex')::uuid, \
                       encode($4, 'hex')::uuid, $5, 1, 1)",
    )
    .bind(uuid_v7(11).as_slice())
    .bind(ACCOUNT)
    .bind(uuid_v7(12).as_slice())
    .bind(uuid_v7(session_raw).as_slice())
    .bind(now + 120)
    .execute(&mut connection)
    .await?;
    // Independent complete history for this positive owning fixture.
    sqlx::query("INSERT INTO game_durability_session_use_ledgers VALUES (encode($1,'hex')::uuid,1,TRUE,1,1)")
            .bind(uuid_v7(11).as_slice()).execute(&mut connection).await?;
    sqlx::query("INSERT INTO game_durability_session_use_memberships VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,$3)")
            .bind(uuid_v7(session_raw).as_slice()).bind(uuid_v7(11).as_slice()).bind([1_u8;16].as_slice()).execute(&mut connection).await?;
    connection.close().await?;
    Ok(())
}

fn predecessor_snapshot(
    current_channel_raw: u64,
) -> Result<GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>, ReconnectDurabilityErrorV1> {
    predecessor_snapshot_with_world_eligibility(
        current_channel_raw,
        Some(CharacterWorldEligibilityClaimV1::new(
            character(11)?,
            world(12)?,
        )),
    )
}

fn predecessor_snapshot_with_world_eligibility(
    current_channel_raw: u64,
    current_character_world_eligibility: Option<CharacterWorldEligibilityClaimV1>,
) -> Result<GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>, ReconnectDurabilityErrorV1> {
    let facts =
        FreshAdmissionFacts::new([0x44; 32], character(11)?, world(12)?, channel(13)?, 9, 10)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let initial_transport = AuthenticatedTransportRefV1::decode(&[0x70; 16])
        .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    let commit = FreshAdmissionCommit::from_facts(game_session(10)?, facts, initial_transport)
        .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
    GameSessionAuthoritySnapshot::from_current_facts(
        commit,
        GameSessionState::Terminal,
        ConnectionGeneration::new(7).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        None,
        CharacterLease::new(character(11)?, 9)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        current_character_world_eligibility,
        RuntimeScopeRefV1::channel(world(12)?, channel(current_channel_raw)?),
        ScopeOwnershipGeneration::new(10).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
    )?
    .with_control_loss_continuity(ControlLossEpochRefV1::new(3)?, 120)
}

fn prepared_flow(
    record: &ReconnectDurabilityRecordV1,
) -> Result<ReconnectDurabilityFlowV2, ReconnectDurabilityErrorV1> {
    let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
    budget.reserve(
        record.identity().reconnect_attempt_ref(),
        record.connection().transport_ref(),
    )?;
    let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
    flow.accept_prepare_completion(
        ReconnectPrepareCompletionV2::for_request(
            &request,
            ReconnectPrepareDispositionV2::Prepared,
        ),
        &mut budget,
    )?;
    Ok(flow)
}

fn current_authority(
    record: &ReconnectDurabilityRecordV1,
    runtime_scope: RuntimeScopeRefV1,
    connection_generation: ConnectionGeneration,
    control_loss_epoch: ControlLossEpochRefV1,
    proof: ReconnectProofV1,
) -> Result<ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1> {
    current_authority_with_bindings(
        record,
        Some(CharacterWorldEligibilityClaimV1::new(
            record.identity().character_id(),
            record.identity().world_id(),
        )),
        Some(ReconnectCandidateBindingV1::new(
            record.identity().game_session_id(),
            record.identity().reconnect_attempt_ref(),
            record.connection().candidate(),
            record.connection().transport_ref(),
            record.continuity().prepared_deadline(),
        )?),
        runtime_scope,
        connection_generation,
        control_loss_epoch,
        proof,
        105,
    )
}

#[allow(clippy::too_many_arguments)]
fn current_authority_with_bindings(
    record: &ReconnectDurabilityRecordV1,
    current_character_world_eligibility: Option<CharacterWorldEligibilityClaimV1>,
    current_candidate: Option<ReconnectCandidateBindingV1>,
    runtime_scope: RuntimeScopeRefV1,
    connection_generation: ConnectionGeneration,
    control_loss_epoch: ControlLossEpochRefV1,
    proof: ReconnectProofV1,
    observed_at: i64,
) -> Result<ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1> {
    ReconnectCurrentAuthorityV1::from_current_facts(
        record,
        Some(AccountPresenceClaimV1::new(
            record.identity().account_id(),
            record.identity().character_id(),
        )?),
        current_character_world_eligibility,
        current_candidate,
        runtime_scope,
        connection_generation,
        record.authority(),
        control_loss_epoch,
        record.continuity().original_grace_deadline(),
        proof,
        record.fnd02().clone(),
        record.compatibility().clone(),
        GameSessionState::Reconnectable,
        false,
        observed_at,
    )
}
