use crate::durability::DurabilityError;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn connect(database_url: &str, max_connections: u32) -> Result<PgPool, DurabilityError> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(CONNECT_TIMEOUT)
        .connect(database_url)
        .await
        .map_err(DurabilityError::from)
}

#[cfg(test)]
mod runtime_scope_identity_red_tests {
    use crate::durability::{AdmissionReconnectJournalV2, MigrationExecutor};
    use oteryn_game_server::foundation::{
        AuthenticatedTransportRefV1, AuthorityEvidenceFenceV1, ChannelId, CharacterId,
        CharacterLease, CommandId, ConnectionGeneration, ControlLossEpochRefV1,
        Fnd02ReconciliationFenceV1, FreshAdmissionCommit, FreshAdmissionFacts,
        GameSessionAuthoritySnapshot, GameSessionId, GameSessionState, ProtectionEntitlementV1,
        ReconnectAttemptBudgetV1, ReconnectAttemptRef, ReconnectAuthorityFenceV1,
        ReconnectCommitDispositionV1, ReconnectCompatibilityEvidenceV1, ReconnectConnectionFenceV1,
        ReconnectContinuityV1, ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1,
        ReconnectDurabilityFlowV1, ReconnectDurabilityFlowV2, ReconnectDurabilityRecordV1,
        ReconnectDurableReconciliationSnapshotV1, ReconnectIdentityV1,
        ReconnectPrepareCompletionV1, ReconnectPrepareCompletionV2, ReconnectPrepareDispositionV1,
        ReconnectPrepareDispositionV2, ReconnectProofV1, RuntimeScopeRefV1,
        ScopeOwnershipGeneration, TerminalGameSessionReplacementAuthorizationV1, WorldId,
    };
    use sqlx::{Connection, Executor, PgConnection};
    use std::error::Error;
    use std::future::Future;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    const ACCOUNT: &str = "123e4567-e89b-12d3-a456-426614174000";
    static DB_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    type TestResult = Result<(), Box<dyn Error>>;

    struct IsolatedDatabase {
        admin_url: String,
        database_name: String,
    }

    impl IsolatedDatabase {
        async fn create(test_name: &str) -> Result<Self, Box<dyn Error>> {
            let admin_url = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
            if !(admin_url.starts_with("postgresql://oteryn_test_admin:")
                && (admin_url.contains("@127.0.0.1:5432/")
                    || admin_url.contains("@localhost:5432/"))
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

    async fn migrated_database(
        test_name: &str,
    ) -> Result<(IsolatedDatabase, String), Box<dyn Error>> {
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
            ScopeOwnershipGeneration::new(10)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
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
            ScopeOwnershipGeneration::new(10)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
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
            RuntimeScopeRefV1::channel(world(12)?, channel(13)?),
            ScopeOwnershipGeneration::new(10)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?
        .with_control_loss_continuity(
            candidate.continuity().control_loss_epoch(),
            candidate.continuity().original_grace_deadline(),
        )?;
        TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
            ACCOUNT,
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
        connection.close().await?;
        Ok(())
    }

    fn predecessor_snapshot(
        current_channel_raw: u64,
    ) -> Result<GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>, ReconnectDurabilityErrorV1>
    {
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
            RuntimeScopeRefV1::channel(world(12)?, channel(current_channel_raw)?),
            ScopeOwnershipGeneration::new(10)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
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
        ReconnectCurrentAuthorityV1::from_current_facts(
            record,
            runtime_scope,
            connection_generation,
            record.authority(),
            control_loss_epoch,
            proof,
            record.fnd02().clone(),
            record.compatibility().clone(),
            GameSessionState::Reconnectable,
            false,
            105,
        )
    }

    #[test]
    fn terminal_replacement_rejects_same_world_generation_different_current_runtime_scope()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let candidate = candidate_record()?;
        let result = TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
            ACCOUNT,
            game_session(10)?,
            game_session(20)?,
            predecessor_snapshot(14)?,
            &candidate,
        );
        assert_eq!(result, Err(ReconnectDurabilityErrorV1::StaleAuthority));
        Ok(())
    }

    #[test]
    fn final_revalidation_requires_complete_current_authority_facts()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let record = candidate_record()?;
        let exact_scope = record.identity().runtime_scope();
        let exact_connection = record.connection().predecessor();
        let exact_epoch = record.continuity().control_loss_epoch();
        let exact_proof = record.proof().clone();

        let mut exact_flow = prepared_flow(&record)?;
        let exact = current_authority(
            &record,
            exact_scope,
            exact_connection,
            exact_epoch,
            exact_proof.clone(),
        )?;
        assert!(exact_flow.authorize_commit(exact, 104).is_ok());

        let mut scope_flow = prepared_flow(&record)?;
        let changed_scope = current_authority(
            &record,
            RuntimeScopeRefV1::channel(record.identity().world_id(), channel(14)?),
            exact_connection,
            exact_epoch,
            exact_proof.clone(),
        )?;
        assert_eq!(
            scope_flow.authorize_commit(changed_scope, 104),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );

        let mut connection_flow = prepared_flow(&record)?;
        let changed_connection = current_authority(
            &record,
            exact_scope,
            ConnectionGeneration::new(9).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            exact_epoch,
            exact_proof.clone(),
        )?;
        assert_eq!(
            connection_flow.authorize_commit(changed_connection, 104),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );

        let mut epoch_flow = prepared_flow(&record)?;
        let changed_epoch = current_authority(
            &record,
            exact_scope,
            exact_connection,
            ControlLossEpochRefV1::new(4)?,
            exact_proof,
        )?;
        assert_eq!(
            epoch_flow.authorize_commit(changed_epoch, 104),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );

        let mut proof_flow = prepared_flow(&record)?;
        let changed_proof = current_authority(
            &record,
            exact_scope,
            exact_connection,
            exact_epoch,
            ReconnectProofV1::ReauthenticatedRecovery {
                recovery_grant_nonce: [0x56; 32],
            },
        )?;
        assert_eq!(
            proof_flow.authorize_commit(changed_proof, 104),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );
        Ok(())
    }

    #[test]
    fn replacement_authorization_rejects_runtime_scope_record_substitution()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let candidate = candidate_record()?;
        let authorization = TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
            ACCOUNT,
            game_session(10)?,
            game_session(20)?,
            predecessor_snapshot(13)?,
            &candidate,
        )?;
        let substituted = candidate_record_for_channel(14)?;

        assert!(
            !crate::durability::replacement_authorization_matches_record(
                &authorization,
                &substituted,
            )
        );
        Ok(())
    }

    #[test]
    fn replacement_reconciliation_requires_receipt_authorization_when_request_omits_it()
    -> TestResult {
        run_postgres_test(async {
            let (database, database_url) =
                migrated_database("unsigned_replacement_reconcile").await?;
            let now = unix_now()?;
            seed_current_actor_anchor(&database_url, 10, now).await?;
            let journal = AdmissionReconnectJournalV2::connect_runtime(&database_url).await?;
            let candidate = postgres_record(now, 20, 1, 0xa1, 7, ProtectionEntitlementV1::unused())
                .map_err(|_| "candidate record")?;
            let authorization = replacement_authorization(&candidate, 10, 7)
                .map_err(|_| "replacement authorization")?;
            let signed_request =
                ReconnectDurabilityFlowV2::begin(candidate.clone(), Some(authorization)).1;
            assert_eq!(
                journal.prepare(&signed_request).await?,
                ReconnectPrepareDispositionV2::Prepared
            );

            let unsigned_request = ReconnectDurabilityFlowV2::begin(candidate, None).1;
            assert!(matches!(
                journal.reconcile(&unsigned_request).await,
                Err(crate::durability::DurabilityError::InvalidStoredState)
            ));

            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn replacement_prepare_requires_receipt_authorization_when_request_omits_it() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) = migrated_database("unsigned_replacement_prepare").await?;
            let now = unix_now()?;
            seed_current_actor_anchor(&database_url, 10, now).await?;
            let journal = AdmissionReconnectJournalV2::connect_runtime(&database_url).await?;
            let candidate = postgres_record(now, 20, 1, 0xa2, 7, ProtectionEntitlementV1::unused())
                .map_err(|_| "candidate record")?;
            let authorization = replacement_authorization(&candidate, 10, 7)
                .map_err(|_| "replacement authorization")?;
            let signed_request =
                ReconnectDurabilityFlowV2::begin(candidate.clone(), Some(authorization)).1;
            assert_eq!(
                journal.prepare(&signed_request).await?,
                ReconnectPrepareDispositionV2::Prepared
            );

            let unsigned_request = ReconnectDurabilityFlowV2::begin(candidate, None).1;
            assert!(matches!(
                journal.prepare(&unsigned_request).await,
                Err(crate::durability::DurabilityError::InvalidStoredState)
            ));

            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn legacy_prepare_rejects_receipt_backed_replacement_candidate() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) = migrated_database("legacy_replacement_prepare").await?;
            let now = unix_now()?;
            seed_current_actor_anchor(&database_url, 10, now).await?;
            let journal = AdmissionReconnectJournalV2::connect_runtime(&database_url).await?;
            let candidate = postgres_record(now, 20, 1, 0xa3, 7, ProtectionEntitlementV1::unused())
                .map_err(|_| "candidate record")?;
            let authorization = replacement_authorization(&candidate, 10, 7)
                .map_err(|_| "replacement authorization")?;
            let signed_request =
                ReconnectDurabilityFlowV2::begin(candidate.clone(), Some(authorization)).1;
            assert_eq!(
                journal.prepare(&signed_request).await?,
                ReconnectPrepareDispositionV2::Prepared
            );

            let (_, legacy_request) = ReconnectDurabilityFlowV1::begin(candidate);
            assert!(matches!(
                journal.legacy().prepare(&legacy_request).await,
                Err(crate::durability::DurabilityError::InvalidStoredState)
            ));

            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn terminal_replacement_preserves_committed_predecessor_reconciliation() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) =
                migrated_database("committed_predecessor_reconcile").await?;
            let now = unix_now()?;
            let journal = AdmissionReconnectJournalV2::connect_runtime(&database_url).await?;
            let fenced = ProtectionEntitlementV1::fenced(42).map_err(|_| "fenced protection")?;
            let predecessor_record =
                postgres_record(now, 10, 1, 0xb1, 7, fenced).map_err(|_| "predecessor record")?;
            let (mut predecessor_flow, predecessor_prepare) =
                ReconnectDurabilityFlowV1::begin(predecessor_record.clone());
            assert_eq!(
                journal.legacy().prepare(&predecessor_prepare).await?,
                ReconnectPrepareDispositionV1::Prepared
            );
            predecessor_flow
                .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &predecessor_prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(|_| "predecessor prepare completion")?;
            let current = ReconnectCurrentAuthorityV1::from_record(&predecessor_record, now)
                .map_err(|_| "predecessor current authority")?;
            let predecessor_commit = predecessor_flow
                .authorize_commit(current, now)
                .map_err(|_| "predecessor commit authorization")?;
            assert_eq!(
                journal.legacy().commit(&predecessor_commit).await?,
                ReconnectCommitDispositionV1::Committed
            );

            let candidate = postgres_record(
                now,
                20,
                2,
                0xb2,
                8,
                ProtectionEntitlementV1::fenced(42).map_err(|_| "candidate protection")?,
            )
            .map_err(|_| "candidate record")?;
            let authorization = replacement_authorization(&candidate, 10, 8)
                .map_err(|_| "replacement authorization")?;
            let replacement_request =
                ReconnectDurabilityFlowV2::begin(candidate, Some(authorization)).1;
            assert_eq!(
                journal.prepare(&replacement_request).await?,
                ReconnectPrepareDispositionV2::Prepared
            );

            assert_eq!(
                journal.legacy().reconcile(&predecessor_prepare).await?,
                ReconnectDurableReconciliationSnapshotV1::terminal(predecessor_record)
            );

            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }
}
