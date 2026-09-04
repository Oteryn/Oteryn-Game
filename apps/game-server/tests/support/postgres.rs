use sqlx::postgres::PgConnectOptions;
use sqlx::{Connection, Executor, PgConnection};
use std::env;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};

static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn current_authority_from_record(
    record: &oteryn_game_server::foundation::ReconnectDurabilityRecordV1,
    observed_at: i64,
) -> Result<
    oteryn_game_server::foundation::ReconnectCurrentAuthorityV1,
    oteryn_game_server::foundation::ReconnectDurabilityErrorV1,
> {
    use oteryn_game_server::foundation::{
        AccountPresenceClaimV1, CharacterWorldEligibilityClaimV1, GameSessionState,
        ReconnectCandidateBindingV1, ReconnectCurrentAuthorityV1,
    };

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

#[derive(Debug)]
pub enum IsolatedPostgresError {
    MissingAdminUrl,
    UnsafeAdminUrl,
    Sqlx(sqlx::Error),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PostgresE2eAvailability {
    NotConfigured,
    Configured,
}

impl Display for IsolatedPostgresError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingAdminUrl => {
                "OTERYN_TEST_POSTGRES_ADMIN_URL is required for PostgreSQL E2E"
            }
            Self::UnsafeAdminUrl => "PostgreSQL E2E requires the pinned local test-admin URL",
            Self::Sqlx(error) => {
                return write!(formatter, "PostgreSQL E2E operation failed: {error}");
            }
        })
    }
}

impl std::error::Error for IsolatedPostgresError {}

impl From<sqlx::Error> for IsolatedPostgresError {
    fn from(error: sqlx::Error) -> Self {
        Self::Sqlx(error)
    }
}

#[derive(Debug)]
pub struct IsolatedPostgres {
    admin_url: String,
    database_name: String,
}

impl IsolatedPostgres {
    pub async fn create(test_name: &str) -> Result<Self, IsolatedPostgresError> {
        let admin_url = env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")
            .map_err(|_error| IsolatedPostgresError::MissingAdminUrl)?;
        validate_admin_url(&admin_url)?;

        let database_name = database_name(test_name)?;
        let mut admin = PgConnection::connect(&admin_url).await?;
        let create_database = format!("CREATE DATABASE {database_name}");
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(create_database)))
            .await?;

        Ok(Self {
            admin_url,
            database_name,
        })
    }

    pub fn database_url(&self) -> Result<String, IsolatedPostgresError> {
        let prefix = self
            .admin_url
            .strip_suffix("/postgres")
            .ok_or(IsolatedPostgresError::UnsafeAdminUrl)?;
        Ok(format!("{prefix}/{}", self.database_name))
    }

    pub async fn cleanup(self) -> Result<(), IsolatedPostgresError> {
        let mut admin = PgConnection::connect(&self.admin_url).await?;
        sqlx::query(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
             WHERE datname = $1 AND pid <> pg_backend_pid()",
        )
        .bind(&self.database_name)
        .execute(&mut admin)
        .await?;
        let drop_database = format!("DROP DATABASE IF EXISTS {}", self.database_name);
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(drop_database)))
            .await?;
        Ok(())
    }
}

pub fn postgres_e2e_availability() -> Result<PostgresE2eAvailability, IsolatedPostgresError> {
    match env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") {
        Ok(admin_url) => classify_e2e_admin_url(Some(&admin_url)),
        Err(env::VarError::NotPresent) => classify_e2e_admin_url(None),
        Err(env::VarError::NotUnicode(_value)) => Err(IsolatedPostgresError::UnsafeAdminUrl),
    }
}

pub fn classify_e2e_admin_url(
    value: Option<&str>,
) -> Result<PostgresE2eAvailability, IsolatedPostgresError> {
    let Some(value) = value else {
        return Ok(PostgresE2eAvailability::NotConfigured);
    };
    validate_admin_url(value)?;
    Ok(PostgresE2eAvailability::Configured)
}

pub fn validate_admin_url(value: &str) -> Result<(), IsolatedPostgresError> {
    if value.contains(['?', '#', '\n', '\r'])
        || env::var_os("PGOPTIONS").is_some()
        || env::var_os("PGHOST").is_some()
        || env::var_os("PGSERVICE").is_some()
        || env::var_os("PGSERVICEFILE").is_some()
    {
        return Err(IsolatedPostgresError::UnsafeAdminUrl);
    }

    let options = PgConnectOptions::from_str(value)
        .map_err(|_error| IsolatedPostgresError::UnsafeAdminUrl)?;
    if !matches!(options.get_host(), "127.0.0.1" | "localhost")
        || options.get_port() != 5432
        || options.get_socket().is_some()
        || options.get_username() != "oteryn_test_admin"
        || options.get_database() != Some("postgres")
        || !value.starts_with("postgresql://oteryn_test_admin:")
        || !value.ends_with("/postgres")
    {
        return Err(IsolatedPostgresError::UnsafeAdminUrl);
    }

    Ok(())
}

fn database_name(test_name: &str) -> Result<String, IsolatedPostgresError> {
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
    if normalized.is_empty() || normalized.len() > 40 {
        return Err(IsolatedPostgresError::UnsafeAdminUrl);
    }

    let ordinal = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let process = std::process::id();
    Ok(format!("oteryn_game_test_{process}_{ordinal}_{normalized}"))
}

#[cfg(test)]
mod durability_contract_tests {
    use super::{
        IsolatedPostgres, IsolatedPostgresError, PostgresE2eAvailability, postgres_e2e_availability,
    };
    use crate::durability::{
        AdmissionReconnectJournal, DurabilityError, MigrationExecutor, SchemaCompatibility,
    };
    use oteryn_game_server::foundation::{
        AuthenticatedTransportRefV1, AuthorityEvidenceFenceV1, ChannelId, CharacterId, CommandId,
        ConnectionGeneration, ControlLossEpochRefV1, Fnd02ReconciliationFenceV1, GameSessionId,
        PendingCommandDispositionV1, PendingCommandReconciliationV1, ProtectionEntitlementV1,
        ReconnectAttemptRef, ReconnectAuthorityFenceV1, ReconnectCommitDispositionV1,
        ReconnectCompatibilityEvidenceV1, ReconnectConnectionFenceV1, ReconnectContinuityV1,
        ReconnectDurabilityErrorV1, ReconnectDurabilityFlowV1, ReconnectDurabilityRecordV1,
        ReconnectIdentityV1, ReconnectPrepareCompletionV1, ReconnectPrepareDispositionV1,
        ReconnectProofV1, RuntimeScopeRefV1, ScopeOwnershipGeneration, StateDomainRevisionV1,
        WorldId,
    };
    use sqlx::{Connection, Executor, PgConnection};
    use std::error::Error;
    use std::future::Future;
    use std::time::Duration;

    type TestResult = Result<(), Box<dyn Error>>;
    type CanonicalAttemptRow = (
        Vec<u8>,
        String,
        Vec<u8>,
        Vec<u8>,
        i16,
        Vec<u8>,
        Option<Vec<u8>>,
        Option<Vec<u8>>,
        String,
    );

    fn run_postgres_test<F>(future: F) -> TestResult
    where
        F: Future<Output = TestResult>,
    {
        if postgres_e2e_availability()? == PostgresE2eAvailability::NotConfigured {
            return Ok(());
        }
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(future)
    }

    async fn migrated_database(
        test_name: &str,
    ) -> Result<(IsolatedPostgres, String, MigrationExecutor), Box<dyn Error>> {
        let database = IsolatedPostgres::create(test_name).await?;
        let database_url = database.database_url()?;
        let executor = MigrationExecutor::connect_migration(&database_url).await?;
        executor.apply_embedded_ledger().await?;
        Ok((database, database_url, executor))
    }

    fn full_range_record(
        now: i64,
    ) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        fn invalid_record<E>(_error: E) -> ReconnectDurabilityErrorV1 {
            ReconnectDurabilityErrorV1::InvalidRecord
        }

        let game_session_id = GameSessionId::decode(&crate::uuid_v7(84)).map_err(invalid_record)?;
        let character_id = CharacterId::decode(&crate::uuid_v7(11)).map_err(invalid_record)?;
        let world_id = WorldId::decode(&crate::uuid_v7(12)).map_err(invalid_record)?;
        let channel_id = ChannelId::decode(&crate::uuid_v7(13)).map_err(invalid_record)?;
        let identity = ReconnectIdentityV1::new(
            game_session_id,
            ReconnectAttemptRef::new(1).map_err(invalid_record)?,
            "123e4567-e89b-12d3-a456-426614174000",
            character_id,
            world_id,
            RuntimeScopeRefV1::channel(world_id, channel_id),
        )?;
        let connection = ReconnectConnectionFenceV1::new(
            ConnectionGeneration::new(u64::MAX - 1).map_err(invalid_record)?,
            ConnectionGeneration::new(u64::MAX).map_err(invalid_record)?,
            AuthenticatedTransportRefV1::decode(&[0xd1; 16]).map_err(invalid_record)?,
        )?;
        let authority = ReconnectAuthorityFenceV1::new(
            u64::MAX - 2,
            ScopeOwnershipGeneration::new(u64::MAX - 3).map_err(invalid_record)?,
        )?;
        let continuity = ReconnectContinuityV1::new(
            ControlLossEpochRefV1::new(u64::MAX - 4)?,
            now + 120,
            now + 115,
            ProtectionEntitlementV1::unused(),
        )?;
        let fnd02 = Fnd02ReconciliationFenceV1::new(
            CommandId::new(u64::MAX).map_err(invalid_record)?,
            vec![
                PendingCommandReconciliationV1::new(
                    CommandId::new(u64::MAX - 2).map_err(invalid_record)?,
                    PendingCommandDispositionV1::PendingOriginal,
                ),
                PendingCommandReconciliationV1::new(
                    CommandId::new(u64::MAX - 1).map_err(invalid_record)?,
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
                recovery_grant_nonce: [0x66; 32],
            },
            fnd02,
            compatibility,
        )
    }

    #[test]
    fn runtime_startup_rejects_missing_ledger_without_creating_ddl() -> TestResult {
        run_postgres_test(async {
            let database = IsolatedPostgres::create("runtime_no_ddl").await?;
            let database_url = database.database_url()?;

            let startup = AdmissionReconnectJournal::connect_runtime(&database_url).await;
            assert!(matches!(
                startup,
                Err(DurabilityError::SchemaIncompatible(
                    SchemaCompatibility::MissingMigrationLedger
                ))
            ));

            let mut connection = PgConnection::connect(&database_url).await?;
            let migration_table: Option<String> =
                sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations')::text")
                    .fetch_one(&mut connection)
                    .await?;
            assert!(migration_table.is_none());
            connection.close().await?;
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn checksum_mismatch_is_runtime_incompatible() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, executor) = migrated_database("checksum_mismatch").await?;
            let mut connection = PgConnection::connect(&database_url).await?;
            sqlx::query("UPDATE _sqlx_migrations SET checksum = decode('00', 'hex')")
                .execute(&mut connection)
                .await?;
            connection.close().await?;
            assert_eq!(executor.inspect().await?, SchemaCompatibility::Incompatible);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn dirty_migration_is_runtime_incompatible() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, executor) = migrated_database("dirty_migration").await?;
            let mut connection = PgConnection::connect(&database_url).await?;
            sqlx::query("UPDATE _sqlx_migrations SET success = false")
                .execute(&mut connection)
                .await?;
            connection.close().await?;
            assert_eq!(executor.inspect().await?, SchemaCompatibility::Incompatible);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn behind_migration_ledger_is_runtime_incompatible() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, executor) = migrated_database("behind_ledger").await?;
            let mut connection = PgConnection::connect(&database_url).await?;
            sqlx::query("DELETE FROM _sqlx_migrations")
                .execute(&mut connection)
                .await?;
            connection.close().await?;
            assert_eq!(executor.inspect().await?, SchemaCompatibility::Incompatible);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn ahead_migration_ledger_is_runtime_incompatible() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, executor) = migrated_database("ahead_ledger").await?;
            let mut connection = PgConnection::connect(&database_url).await?;
            sqlx::query(
                "INSERT INTO _sqlx_migrations \
                 (version, description, success, checksum, execution_time) \
                 SELECT version + 1000000, 'synthetic ahead migration', true, checksum, 0 \
                 FROM _sqlx_migrations ORDER BY version DESC LIMIT 1",
            )
            .execute(&mut connection)
            .await?;
            connection.close().await?;
            assert_eq!(executor.inspect().await?, SchemaCompatibility::Incompatible);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn migration_lock_interruption_releases_before_any_ddl_and_allows_fresh_retry() -> TestResult {
        run_postgres_test(async {
            let database = IsolatedPostgres::create("migration_lock_interrupt").await?;
            let database_url = database.database_url()?;
            let migration_lock_id = sqlx_migration_lock_id(&database.database_name);
            let mut lock_holder = PgConnection::connect(&database_url).await?;
            sqlx::query("SELECT pg_advisory_lock($1)")
                .bind(migration_lock_id)
                .execute(&mut lock_holder)
                .await?;
            let executor = MigrationExecutor::connect_migration(&database_url).await?;
            assert!(
                tokio::time::timeout(Duration::from_millis(250), executor.apply_embedded_ledger())
                    .await
                    .is_err()
            );
            let mut observer = PgConnection::connect(&database_url).await?;
            let migration_table: Option<String> =
                sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations')::text")
                    .fetch_one(&mut observer)
                    .await?;
            assert!(migration_table.is_none());
            observer.close().await?;
            lock_holder.close().await?;
            tokio::time::timeout(Duration::from_secs(5), executor.apply_embedded_ledger())
                .await??;
            assert_eq!(executor.inspect().await?, SchemaCompatibility::Compatible);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn isolated_database_outage_fails_closed_and_runtime_recovers() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, executor) = migrated_database("database_outage").await?;
            drop(executor);
            let mut admin = PgConnection::connect(&database.admin_url).await?;
            set_accepting_connections(&mut admin, &database.database_name, false).await?;
            sqlx::query(
                "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
                 WHERE datname = $1 AND pid <> pg_backend_pid()",
            )
            .bind(&database.database_name)
            .execute(&mut admin)
            .await?;
            let outage = AdmissionReconnectJournal::connect_runtime(&database_url).await;
            assert!(matches!(outage, Err(DurabilityError::Database(_))));
            set_accepting_connections(&mut admin, &database.database_name, true).await?;
            let recovered = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            drop(recovered);
            admin.close().await?;
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn rejected_unseen_epoch_is_retained_for_same_attempt_replay() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, _executor) =
                migrated_database("unseen_epoch_stale").await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let record_now = crate::unix_now().map_err(crate::foundation_error)?;
            let (_first_flow, first_prepare) = ReconnectDurabilityFlowV1::begin(
                crate::record(89, 1, 0xd6, record_now).map_err(crate::foundation_error)?,
            );
            assert_eq!(
                journal.prepare(&first_prepare).await?,
                ReconnectPrepareDispositionV1::Prepared
            );
            let (_stale_epoch_flow, stale_epoch) = ReconnectDurabilityFlowV1::begin(
                crate::record_for_epoch(89, 2, 0xd7, record_now, record_now + 115, 4, 7, 8, 0x67)
                    .map_err(crate::foundation_error)?,
            );
            assert_eq!(
                journal.prepare(&stale_epoch).await?,
                ReconnectPrepareDispositionV1::RejectedStaleAuthority
            );
            assert_eq!(
                journal.prepare(&stale_epoch).await?,
                ReconnectPrepareDispositionV1::ExistingTerminal
            );
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn reconcile_without_durable_session_is_invalid_stored_state() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, _executor) =
                migrated_database("missing_reconcile_session").await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let record_now = crate::unix_now().map_err(crate::foundation_error)?;
            let (_flow, prepare) = ReconnectDurabilityFlowV1::begin(
                crate::record(92, 1, 0xd8, record_now).map_err(crate::foundation_error)?,
            );
            assert!(matches!(
                journal.reconcile(&prepare).await,
                Err(DurabilityError::InvalidStoredState)
            ));
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn commit_without_durable_session_is_rejected_stale_authority() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, _executor) =
                migrated_database("missing_commit_session").await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let record_now = crate::unix_now().map_err(crate::foundation_error)?;
            let (mut flow, prepare) = ReconnectDurabilityFlowV1::begin(
                crate::record(88, 1, 0xd5, record_now).map_err(crate::foundation_error)?,
            );
            flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                &prepare,
                ReconnectPrepareDispositionV1::Prepared,
            ))
            .map_err(crate::foundation_error)?;
            let current = super::current_authority_from_record(prepare.record(), record_now)
                .map_err(crate::foundation_error)?;
            let commit = flow
                .authorize_commit(current, record_now)
                .map_err(crate::foundation_error)?;
            assert_eq!(
                journal.commit(&commit).await?,
                ReconnectCommitDispositionV1::RejectedStaleAuthority
            );
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn recovery_grant_nonce_is_single_consumed_at_commit() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, _executor) = migrated_database("recovery_nonce").await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let record_now = crate::unix_now().map_err(crate::foundation_error)?;
            let (mut first_flow, first_prepare) = ReconnectDurabilityFlowV1::begin(
                crate::record(80, 1, 0x81, record_now).map_err(crate::foundation_error)?,
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
                .map_err(crate::foundation_error)?;
            let first_current =
                super::current_authority_from_record(first_prepare.record(), record_now)
                    .map_err(crate::foundation_error)?;
            let first_commit = first_flow
                .authorize_commit(first_current, record_now)
                .map_err(crate::foundation_error)?;
            assert_eq!(
                journal.commit(&first_commit).await?,
                ReconnectCommitDispositionV1::Committed
            );

            let (mut second_flow, second_prepare) = ReconnectDurabilityFlowV1::begin(
                crate::record_for_actor(81, 181, 1, 0x82, record_now)
                    .map_err(crate::foundation_error)?,
            );
            assert_eq!(
                journal.prepare(&second_prepare).await?,
                ReconnectPrepareDispositionV1::Prepared
            );
            second_flow
                .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &second_prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(crate::foundation_error)?;
            let second_current =
                super::current_authority_from_record(second_prepare.record(), record_now)
                    .map_err(crate::foundation_error)?;
            let second_commit = second_flow
                .authorize_commit(second_current, record_now)
                .map_err(crate::foundation_error)?;
            assert_eq!(
                journal.commit(&second_commit).await?,
                ReconnectCommitDispositionV1::RejectedStaleAuthority
            );
            let mut connection = PgConnection::connect(&database_url).await?;
            let consumed: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game_durability_recovery_grant_consumptions",
            )
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(consumed, 1);
            connection.close().await?;
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn stale_prepare_attempts_consume_the_retained_attempt_bound() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, _executor) = migrated_database("stale_capacity").await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let record_now = crate::unix_now().map_err(crate::foundation_error)?;
            let (_first_flow, first_prepare) = ReconnectDurabilityFlowV1::begin(
                crate::record(82, 1, 0x91, record_now).map_err(crate::foundation_error)?,
            );
            assert_eq!(
                journal.prepare(&first_prepare).await?,
                ReconnectPrepareDispositionV1::Prepared
            );
            let session_id = first_prepare
                .record()
                .identity()
                .game_session_id()
                .as_bytes()
                .to_vec();
            let mut connection = PgConnection::connect(&database_url).await?;
            sqlx::query(
                "UPDATE game_durability_reconnect_sessions SET predecessor_generation = predecessor_generation + 1 \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(session_id.as_slice())
            .execute(&mut connection)
            .await?;
            for attempt in 2_u64..=8 {
                let transport = u8::try_from(0x90_u64 + attempt)?;
                let (_flow, request) = ReconnectDurabilityFlowV1::begin(
                    crate::record(82, attempt, transport, record_now)
                        .map_err(crate::foundation_error)?,
                );
                assert_eq!(
                    journal.prepare(&request).await?,
                    ReconnectPrepareDispositionV1::RejectedStaleAuthority
                );
            }
            let (_ninth_flow, ninth) = ReconnectDurabilityFlowV1::begin(
                crate::record(82, 9, 0x99, record_now).map_err(crate::foundation_error)?,
            );
            assert_eq!(
                journal.prepare(&ninth).await?,
                ReconnectPrepareDispositionV1::AttemptCapacityExceeded
            );
            let attempt_count: i16 = sqlx::query_scalar(
                "SELECT attempt_count FROM game_durability_reconnect_sessions WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(session_id.as_slice())
            .fetch_one(&mut connection)
            .await?;
            let retained_rows: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game_durability_reconnect_attempts WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(session_id.as_slice())
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(attempt_count, 8);
            assert_eq!(retained_rows, 8);
            connection.close().await?;
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn prepare_requires_reconnectable_state_and_no_current_controller() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, _executor) =
                migrated_database("prepare_authority").await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let record_now = crate::unix_now().map_err(crate::foundation_error)?;
            let (_first_flow, first_prepare) = ReconnectDurabilityFlowV1::begin(
                crate::record(83, 1, 0xa1, record_now).map_err(crate::foundation_error)?,
            );
            assert_eq!(
                journal.prepare(&first_prepare).await?,
                ReconnectPrepareDispositionV1::Prepared
            );
            let session_id = first_prepare
                .record()
                .identity()
                .game_session_id()
                .as_bytes()
                .to_vec();
            let mut connection = PgConnection::connect(&database_url).await?;
            sqlx::query(
                "UPDATE game_durability_reconnect_sessions SET prepared_attempt_ref = NULL, current_transport_ref = $2 \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(session_id.as_slice())
            .bind([0xee_u8; 16].as_slice())
            .execute(&mut connection)
            .await?;
            let (_controller_flow, controller_present) = ReconnectDurabilityFlowV1::begin(
                crate::record(83, 2, 0xa2, record_now).map_err(crate::foundation_error)?,
            );
            assert_eq!(
                journal.prepare(&controller_present).await?,
                ReconnectPrepareDispositionV1::RejectedStaleAuthority
            );
            sqlx::query(
                "UPDATE game_durability_reconnect_sessions SET current_transport_ref = NULL, session_state = 2 \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(session_id.as_slice())
            .execute(&mut connection)
            .await?;
            let (_active_flow, active_session) = ReconnectDurabilityFlowV1::begin(
                crate::record(83, 3, 0xa3, record_now).map_err(crate::foundation_error)?,
            );
            assert_eq!(
                journal.prepare(&active_session).await?,
                ReconnectPrepareDispositionV1::RejectedStaleAuthority
            );
            connection.close().await?;
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn canonical_uuid_and_full_range_command_ids_round_trip() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, _executor) =
                migrated_database("canonical_encoding").await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let record_now = crate::unix_now().map_err(crate::foundation_error)?;
            let record = full_range_record(record_now).map_err(crate::foundation_error)?;
            let (_flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
            assert_eq!(
                journal.prepare(&request).await?,
                ReconnectPrepareDispositionV1::Prepared
            );
            let session_id = record.identity().game_session_id().as_bytes().to_vec();
            let attempt_ref = record
                .identity()
                .reconnect_attempt_ref()
                .to_be_bytes()
                .to_vec();
            let mut connection = PgConnection::connect(&database_url).await?;
            let session_type: String = sqlx::query_scalar(
                "SELECT data_type FROM information_schema.columns WHERE table_schema = 'public' \
                   AND table_name = 'game_durability_reconnect_sessions' AND column_name = 'game_session_id'",
            )
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(session_type, "uuid");
            let uuid_columns: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM information_schema.columns WHERE table_schema = 'public' \
                   AND table_name = 'game_durability_reconnect_attempts' \
                   AND column_name IN ('game_session_id','account_id','character_id','world_id','runtime_scope_world_id','runtime_scope_channel_id','runtime_scope_instance_id') \
                   AND data_type = 'uuid'",
            )
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(uuid_columns, 7);
            let next_command_schema: (String, Option<i32>, Option<i32>) = sqlx::query_as(
                "SELECT data_type, numeric_precision::int, numeric_scale::int FROM information_schema.columns \
                 WHERE table_schema = 'public' AND table_name = 'game_durability_reconnect_attempts' \
                   AND column_name = 'fnd02_next_command_id'",
            )
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(
                next_command_schema,
                ("numeric".to_owned(), Some(20), Some(0))
            );
            let pending_schema: (String, Option<i32>, Option<i32>) = sqlx::query_as(
                "SELECT data_type, numeric_precision::int, numeric_scale::int FROM information_schema.columns \
                 WHERE table_schema = 'public' AND table_name = 'game_durability_reconnect_pending_commands' \
                   AND column_name = 'command_id'",
            )
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(pending_schema, ("numeric".to_owned(), Some(20), Some(0)));
            let session_fences: (String, String, String, String, String) = sqlx::query_as(
                "SELECT control_loss_epoch::text, predecessor_generation::text, \
                        character_lease_generation::text, scope_ownership_generation::text, \
                        current_generation::text \
                 FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(session_id.as_slice())
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(session_fences.0, (u64::MAX - 4).to_string());
            assert_eq!(session_fences.1, (u64::MAX - 1).to_string());
            assert_eq!(session_fences.2, (u64::MAX - 2).to_string());
            assert_eq!(session_fences.3, (u64::MAX - 3).to_string());
            assert_eq!(session_fences.4, (u64::MAX - 1).to_string());
            let stored: CanonicalAttemptRow = sqlx::query_as(
                "SELECT uuid_send(game_session_id), account_id::text, uuid_send(character_id), uuid_send(world_id), \
                        runtime_scope_kind, uuid_send(runtime_scope_world_id), \
                        CASE WHEN runtime_scope_channel_id IS NULL THEN NULL ELSE uuid_send(runtime_scope_channel_id) END, \
                        CASE WHEN runtime_scope_instance_id IS NULL THEN NULL ELSE uuid_send(runtime_scope_instance_id) END, \
                        fnd02_next_command_id::text \
                 FROM game_durability_reconnect_attempts \
                 WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
            )
            .bind(session_id.as_slice())
            .bind(attempt_ref.as_slice())
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(stored.0, crate::uuid_v7(84));
            assert_eq!(stored.1, "123e4567-e89b-12d3-a456-426614174000");
            assert_eq!(stored.2, crate::uuid_v7(11));
            assert_eq!(stored.3, crate::uuid_v7(12));
            assert_eq!(stored.4, 1);
            assert_eq!(stored.5, crate::uuid_v7(12));
            assert_eq!(stored.6.as_deref(), Some(crate::uuid_v7(13).as_slice()));
            assert!(stored.7.is_none());
            assert_eq!(stored.8, u64::MAX.to_string());
            let pending: Vec<(String, i16)> = sqlx::query_as(
                "SELECT command_id::text, disposition FROM game_durability_reconnect_pending_commands \
                 WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2 ORDER BY command_id ASC",
            )
            .bind(session_id.as_slice())
            .bind(attempt_ref.as_slice())
            .fetch_all(&mut connection)
            .await?;
            assert_eq!(
                pending,
                vec![
                    ((u64::MAX - 2).to_string(), 1),
                    ((u64::MAX - 1).to_string(), 2)
                ]
            );
            assert_eq!(
                journal.prepare(&request).await?,
                ReconnectPrepareDispositionV1::ExistingPrepared
            );
            connection.close().await?;
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn durable_session_actor_binding_fails_closed_when_typed_state_is_corrupt() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, _executor) =
                migrated_database("session_actor_corrupt").await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let record_now = crate::unix_now().map_err(crate::foundation_error)?;
            let (_flow, request) = ReconnectDurabilityFlowV1::begin(
                crate::record(86, 1, 0xd3, record_now).map_err(crate::foundation_error)?,
            );
            assert_eq!(
                journal.prepare(&request).await?,
                ReconnectPrepareDispositionV1::Prepared
            );
            let session_id = request
                .record()
                .identity()
                .game_session_id()
                .as_bytes()
                .to_vec();
            let mut connection = PgConnection::connect(&database_url).await?;
            sqlx::query(
                "UPDATE game_durability_reconnect_sessions SET character_id = encode($2, 'hex')::uuid \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(session_id.as_slice())
            .bind(crate::uuid_v7(99).as_slice())
            .execute(&mut connection)
            .await?;
            assert!(matches!(
                journal.prepare(&request).await,
                Err(DurabilityError::InvalidStoredState)
            ));
            connection.close().await?;
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn typed_attempt_mirrors_fail_closed_when_canonical_record_is_unchanged() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, _executor) =
                migrated_database("typed_attempt_corrupt").await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let record_now = crate::unix_now().map_err(crate::foundation_error)?;
            let (_flow, request) = ReconnectDurabilityFlowV1::begin(
                crate::record(87, 1, 0xd4, record_now).map_err(crate::foundation_error)?,
            );
            assert_eq!(
                journal.prepare(&request).await?,
                ReconnectPrepareDispositionV1::Prepared
            );
            let session_id = request
                .record()
                .identity()
                .game_session_id()
                .as_bytes()
                .to_vec();
            let attempt_ref = request
                .record()
                .identity()
                .reconnect_attempt_ref()
                .to_be_bytes()
                .to_vec();
            let mut connection = PgConnection::connect(&database_url).await?;
            sqlx::query(
                "UPDATE game_durability_reconnect_attempts SET control_loss_epoch = control_loss_epoch + 1 \
                 WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
            )
            .bind(session_id.as_slice())
            .bind(attempt_ref.as_slice())
            .execute(&mut connection)
            .await?;
            assert!(matches!(
                journal.prepare(&request).await,
                Err(DurabilityError::InvalidStoredState)
            ));
            sqlx::query(
                "UPDATE game_durability_reconnect_attempts SET control_loss_epoch = control_loss_epoch - 1, \
                        transport_ref = $3 \
                 WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
            )
            .bind(session_id.as_slice())
            .bind(attempt_ref.as_slice())
            .bind([0xee_u8; 16].as_slice())
            .execute(&mut connection)
            .await?;
            assert!(matches!(
                journal.prepare(&request).await,
                Err(DurabilityError::InvalidStoredState)
            ));
            connection.close().await?;
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn committed_reconcile_revalidates_full_current_authority() -> TestResult {
        run_postgres_test(async {
            let (database, database_url, _executor) =
                migrated_database("phase_d_authority").await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let record_now = crate::unix_now().map_err(crate::foundation_error)?;
            let (mut flow, prepare) = ReconnectDurabilityFlowV1::begin(
                crate::record(85, 1, 0xd2, record_now).map_err(crate::foundation_error)?,
            );
            assert_eq!(
                journal.prepare(&prepare).await?,
                ReconnectPrepareDispositionV1::Prepared
            );
            flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                &prepare,
                ReconnectPrepareDispositionV1::Prepared,
            ))
            .map_err(crate::foundation_error)?;
            let current = super::current_authority_from_record(prepare.record(), record_now)
                .map_err(crate::foundation_error)?;
            let commit = flow
                .authorize_commit(current, record_now)
                .map_err(crate::foundation_error)?;
            assert_eq!(
                journal.commit(&commit).await?,
                ReconnectCommitDispositionV1::Committed
            );
            let session_id = prepare
                .record()
                .identity()
                .game_session_id()
                .as_bytes()
                .to_vec();
            let mut connection = PgConnection::connect(&database_url).await?;
            let committed = || {
                oteryn_game_server::foundation::ReconnectDurableReconciliationSnapshotV1::committed(
                    prepare.record().clone(),
                )
            };
            assert_eq!(journal.reconcile(&prepare).await?, committed());
            for (mutation, restore) in [
                ("session_state = 1", "session_state = 2"),
                ("control_loss_epoch = 4", "control_loss_epoch = 3"),
                (
                    "character_lease_generation = 10",
                    "character_lease_generation = 9",
                ),
                (
                    "scope_ownership_generation = 11",
                    "scope_ownership_generation = 10",
                ),
                ("predecessor_generation = 6", "predecessor_generation = 7"),
            ] {
                let mutate = format!(
                    "UPDATE game_durability_reconnect_sessions SET {mutation} WHERE game_session_id = encode($1, 'hex')::uuid"
                );
                connection
                    .execute(sqlx::query(sqlx::AssertSqlSafe(mutate)).bind(session_id.as_slice()))
                    .await?;
                assert!(matches!(
                    journal.reconcile(&prepare).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                let restore = format!(
                    "UPDATE game_durability_reconnect_sessions SET {restore} WHERE game_session_id = encode($1, 'hex')::uuid"
                );
                connection
                    .execute(sqlx::query(sqlx::AssertSqlSafe(restore)).bind(session_id.as_slice()))
                    .await?;
            }
            assert_eq!(journal.reconcile(&prepare).await?, committed());
            connection.close().await?;
            database.cleanup().await?;
            Ok(())
        })
    }

    async fn set_accepting_connections(
        admin: &mut PgConnection,
        database_name: &str,
        accepting: bool,
    ) -> Result<(), IsolatedPostgresError> {
        let value = if accepting { "true" } else { "false" };
        let statement = format!("ALTER DATABASE {database_name} WITH ALLOW_CONNECTIONS {value}");
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(statement)))
            .await?;
        Ok(())
    }

    fn sqlx_migration_lock_id(database_name: &str) -> i64 {
        0x3d32_ad9e_i64 * i64::from(crc32_iso_hdlc(database_name.as_bytes()))
    }

    fn crc32_iso_hdlc(bytes: &[u8]) -> u32 {
        let mut checksum = u32::MAX;
        for &byte in bytes {
            checksum ^= u32::from(byte);
            for _ in 0..8 {
                let low_bit_mask = 0_u32.wrapping_sub(checksum & 1);
                checksum = (checksum >> 1) ^ (0xedb8_8320 & low_bit_mask);
            }
        }
        !checksum
    }
}
