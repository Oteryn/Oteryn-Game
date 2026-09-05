use crate::durability::{DurabilityError, db};
use sqlx::{PgPool, Row};

pub(crate) static GAME_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaCompatibility {
    Compatible,
    MissingMigrationLedger,
    Incompatible,
}

pub struct MigrationExecutor {
    pool: PgPool,
}

impl MigrationExecutor {
    pub async fn connect_migration(database_url: &str) -> Result<Self, DurabilityError> {
        Ok(Self {
            pool: db::connect(database_url, 1).await?,
        })
    }

    /// This is the only runtime path in this module allowed to execute DDL.
    /// Normal game-server startup must use `inspect` through `connect_runtime`.
    pub async fn apply_embedded_ledger(&self) -> Result<(), DurabilityError> {
        GAME_MIGRATOR
            .run(&self.pool)
            .await
            .map_err(DurabilityError::from)
    }

    pub async fn inspect(&self) -> Result<SchemaCompatibility, DurabilityError> {
        inspect(&self.pool).await
    }
}

pub(crate) async fn connect_runtime(database_url: &str) -> Result<PgPool, DurabilityError> {
    let pool = db::connect(database_url, 4).await?;
    let compatibility = inspect(&pool).await?;
    if compatibility != SchemaCompatibility::Compatible {
        return Err(DurabilityError::SchemaIncompatible(compatibility));
    }
    Ok(pool)
}

async fn inspect(pool: &PgPool) -> Result<SchemaCompatibility, DurabilityError> {
    let rows = match sqlx::query(
        "SELECT version, checksum, success FROM _sqlx_migrations ORDER BY version ASC",
    )
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(error) if is_missing_table(&error) => {
            return Ok(SchemaCompatibility::MissingMigrationLedger);
        }
        Err(error) => return Err(DurabilityError::from(error)),
    };

    let expected: Vec<_> = GAME_MIGRATOR.iter().collect();
    if rows.len() != expected.len() {
        return Ok(SchemaCompatibility::Incompatible);
    }

    for (row, migration) in rows.iter().zip(expected) {
        let version: i64 = row.try_get("version")?;
        let checksum: Vec<u8> = row.try_get("checksum")?;
        let success: bool = row.try_get("success")?;
        if !success
            || version != migration.version
            || checksum.as_slice() != migration.checksum.as_ref()
        {
            return Ok(SchemaCompatibility::Incompatible);
        }
    }

    Ok(SchemaCompatibility::Compatible)
}

fn is_missing_table(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|database| database.code())
        .is_some_and(|code| code == "42P01")
}

#[cfg(test)]
mod contract_tests {
    const MIGRATION: &str = include_str!("../../migrations/0001_admission_reconnect_journal.sql");
    const ADMISSION_RECOVERY: &str = include_str!("../foundation/admission_recovery_inner.rs");

    fn session_schema() -> Option<&'static str> {
        MIGRATION
            .split_once("CREATE TABLE game_durability_transport_ref_reservations")
            .map(|(session, _rest)| session)
    }

    #[test]
    fn reconnect_attempts_index_supports_actor_epoch_budget_count() {
        assert!(
            MIGRATION.contains(
                "ON game_durability_reconnect_attempts (character_id, control_loss_epoch)"
            )
        );
    }

    #[test]
    fn record_derived_current_authority_is_test_only_but_current_facts_remain_public() {
        assert!(
            ADMISSION_RECOVERY.contains("pub fn from_current_facts("),
            "production callers must be able to supply independently observed current authority"
        );
        assert!(
            ADMISSION_RECOVERY.contains("#[cfg(test)]\n    pub fn from_record("),
            "record-derived current authority must not be production-accessible"
        );
    }

    #[test]
    fn record_derived_candidate_binding_is_internal_and_its_convenience_is_test_only() {
        let candidate_impl = ADMISSION_RECOVERY
            .split_once("impl ReconnectCandidateBindingV1 {")
            .and_then(|(_before, rest)| rest.split_once("\n}\n"))
            .map(|(implementation, _after)| implementation);
        assert!(
            candidate_impl.is_some(),
            "candidate binding implementation must remain present"
        );
        let Some(candidate_impl) = candidate_impl else {
            return;
        };
        let production_candidate_impl =
            candidate_impl.replace("#[cfg(test)]\n    pub fn from_record(", "");

        assert!(
            !production_candidate_impl.contains("\n    pub fn from_record("),
            "record-derived candidate binding must not be production-public"
        );
        assert!(
            candidate_impl.contains("fn expected_binding_from_record("),
            "immutable record derivation must remain an internal expected-binding helper"
        );
        assert!(
            candidate_impl.contains("#[cfg(test)]\n    pub fn from_record("),
            "the record-derived candidate convenience must remain available only to tests"
        );
    }

    #[test]
    fn identity_derived_authority_claim_convenience_is_test_only_across_sibling_family() {
        for type_name in ["CharacterWorldEligibilityClaimV1", "AccountPresenceClaimV1"] {
            let marker = format!("impl {type_name} {{");
            let implementation = ADMISSION_RECOVERY
                .split_once(&marker)
                .and_then(|(_before, rest)| rest.split_once("\n}\n"))
                .map(|(implementation, _after)| implementation);
            assert!(
                implementation.is_some(),
                "{type_name}: implementation missing"
            );
            let Some(implementation) = implementation else {
                continue;
            };
            let production_impl = implementation
                .replace(
                    "#[cfg(test)]\n    #[must_use]\n    pub fn from_identity(",
                    "",
                )
                .replace("#[cfg(test)]\n    pub fn from_identity(", "");

            assert!(
                !production_impl.contains("\n    pub fn from_identity("),
                "{type_name}: identity-derived authority convenience must not be production-public"
            );
            assert!(
                implementation.contains("fn expected_from_identity("),
                "{type_name}: immutable identity derivation must remain an internal expected-value helper"
            );
            assert!(
                implementation.contains("#[cfg(test)]\n    pub fn from_identity(")
                    || implementation
                        .contains("#[cfg(test)]\n    #[must_use]\n    pub fn from_identity("),
                "{type_name}: identity-derived convenience must remain available only to tests"
            );
        }
    }

    #[test]
    fn generic_v1_terminal_reconciliation_is_not_a_production_recovery_api() {
        const ADMISSION_JOURNAL: &str = include_str!("admission_journal.rs");
        const DURABILITY: &str = include_str!("../durability/mod.rs");

        assert!(
            ADMISSION_JOURNAL.contains("pub(crate) async fn reconcile("),
            "generic V1 reconciliation collapses typed terminal reasons and must remain crate-internal"
        );
        assert!(
            DURABILITY.contains("pub(crate) const fn legacy("),
            "the V2 legacy adapter must not expose generic V1 terminal reconciliation to production callers"
        );
        assert!(
            DURABILITY.contains("pub async fn reconcile("),
            "typed V2 reconciliation must remain the public production recovery boundary"
        );
    }

    #[test]
    fn reconnect_session_schema_binds_actor_and_runtime_scope_identity() {
        let session = session_schema();
        assert!(
            session.is_some(),
            "session table must precede transport reservations"
        );
        let Some(session) = session else {
            return;
        };
        for required in [
            "account_id UUID NOT NULL",
            "character_id UUID NOT NULL",
            "world_id UUID NOT NULL",
            "runtime_scope_kind SMALLINT NOT NULL",
            "runtime_scope_world_id UUID NOT NULL",
            "runtime_scope_channel_id UUID NULL",
            "runtime_scope_instance_id UUID NULL",
        ] {
            assert!(
                session.contains(required),
                "reconnect session schema must retain {required}"
            );
        }
    }

    #[test]
    fn reconnect_authority_fences_preserve_the_full_unsigned_range() {
        for required in [
            "predecessor_generation NUMERIC(20, 0) NOT NULL",
            "character_lease_generation NUMERIC(20, 0) NOT NULL",
            "scope_ownership_generation NUMERIC(20, 0) NOT NULL",
            "current_generation NUMERIC(20, 0) NOT NULL",
        ] {
            assert!(
                MIGRATION.contains(required),
                "reconnect session schema must retain full u64 range for {required}"
            );
        }
        assert_eq!(
            MIGRATION
                .matches("control_loss_epoch NUMERIC(20, 0) NOT NULL")
                .count(),
            3,
            "session, actor-wide continuity and attempt ControlLossEpoch mirrors must retain the full u64 range"
        );
    }
}

#[cfg(test)]
mod terminal_replacement_postgres_red_tests {
    use super::MigrationExecutor;
    use crate::durability::{
        AdmissionReconnectJournal as LegacyAdmissionReconnectJournal, AdmissionReconnectJournalV2,
        DurabilityError,
    };
    use oteryn_game_server::foundation::{
        AccountPresenceClaimV1, AuthenticatedTransportRefV1, AuthorityEvidenceFenceV1, ChannelId,
        CharacterId, CharacterLease, CharacterWorldEligibilityClaimV1, CommandId,
        ConnectionGeneration, ControlLossEpochRefV1, Fnd02ReconciliationFenceV1,
        FreshAdmissionCommit, FreshAdmissionFacts, GameSessionAuthoritySnapshot, GameSessionId,
        GameSessionState, PendingCommandDispositionV1, PendingCommandReconciliationV1,
        ProtectionEntitlementV1, ReconnectAttemptBudgetV1, ReconnectAttemptRef,
        ReconnectAuthorityFenceV1, ReconnectCandidateBindingV1, ReconnectCommitDispositionV1,
        ReconnectCompatibilityEvidenceV1, ReconnectConnectionFenceV1, ReconnectContinuityV1,
        ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1, ReconnectDurabilityFlowV1,
        ReconnectDurabilityFlowV2, ReconnectDurabilityRecordV1, ReconnectDurableOutcomeV2,
        ReconnectDurableReconciliationSnapshotV1, ReconnectDurableReconciliationSnapshotV2,
        ReconnectDurableTerminalDispositionV1, ReconnectIdentityV1, ReconnectPrepareCompletionV1,
        ReconnectPrepareCompletionV2, ReconnectPrepareDispositionV1, ReconnectPrepareDispositionV2,
        ReconnectPrepareRequestV2, ReconnectProofV1, RuntimeScopeRefV1, ScopeOwnershipGeneration,
        StateDomainRevisionV1, TerminalGameSessionReplacementAuthorizationV1, WorldId,
    };
    use sqlx::{Connection, Executor, PgConnection};
    use std::error::Error;
    use std::future::Future;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

    #[derive(Clone)]
    struct AdmissionReconnectJournal {
        v2: AdmissionReconnectJournalV2,
    }

    impl AdmissionReconnectJournal {
        async fn connect_runtime(database_url: &str) -> Result<Self, DurabilityError> {
            Ok(Self {
                v2: AdmissionReconnectJournalV2::connect_runtime(database_url).await?,
            })
        }

        async fn prepare(
            &self,
            request: &oteryn_game_server::foundation::ReconnectPrepareRequestV1,
        ) -> Result<ReconnectPrepareDispositionV1, DurabilityError> {
            self.v2.legacy().prepare(request).await
        }

        async fn commit(
            &self,
            request: &oteryn_game_server::foundation::ReconnectCommitRequestV1,
        ) -> Result<ReconnectCommitDispositionV1, DurabilityError> {
            self.v2.legacy().commit(request).await
        }

        async fn prepare_v2(
            &self,
            request: &ReconnectPrepareRequestV2,
        ) -> Result<ReconnectPrepareDispositionV2, DurabilityError> {
            self.v2.prepare(request).await
        }

        async fn reconcile_v2(
            &self,
            request: &ReconnectPrepareRequestV2,
        ) -> Result<ReconnectDurableReconciliationSnapshotV2, DurabilityError> {
            self.v2.reconcile(request).await
        }
    }

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
                "oteryn_game_repl_{}_{}_{}",
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

    fn unix_now() -> Result<i64, ReconnectDurabilityErrorV1> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?
            .as_secs()
            .try_into()
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
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

    #[allow(clippy::too_many_arguments)]
    fn record(
        game_session_raw: u64,
        character_raw: u64,
        attempt_raw: u64,
        transport_byte: u8,
        epoch: u64,
        predecessor_generation: u64,
        scope_generation: u64,
        now: i64,
    ) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        let world_id = world(12)?;
        let identity = ReconnectIdentityV1::new(
            game_session(game_session_raw)?,
            ReconnectAttemptRef::new(attempt_raw)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            ACCOUNT,
            character(character_raw)?,
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
            ScopeOwnershipGeneration::new(scope_generation)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?;
        let continuity = ReconnectContinuityV1::new(
            ControlLossEpochRefV1::new(epoch)?,
            now + 120,
            now + 115,
            ProtectionEntitlementV1::unused(),
        )?;
        let fnd02 = Fnd02ReconciliationFenceV1::new(
            CommandId::new(3).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            vec![
                PendingCommandReconciliationV1::new(
                    CommandId::new(1).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
                    PendingCommandDispositionV1::PendingOriginal,
                ),
                PendingCommandReconciliationV1::new(
                    CommandId::new(2).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
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
                recovery_grant_nonce: [transport_byte; 32],
            },
            fnd02,
            compatibility,
        )
    }

    fn record_with_protection(
        record: ReconnectDurabilityRecordV1,
        protection_entitlement: ProtectionEntitlementV1,
    ) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        let continuity = ReconnectContinuityV1::new(
            record.continuity().control_loss_epoch(),
            record.continuity().original_grace_deadline(),
            record.continuity().prepared_deadline(),
            protection_entitlement,
        )?;
        ReconnectDurabilityRecordV1::new(
            record.identity().clone(),
            record.connection(),
            record.authority(),
            continuity,
            record.proof().clone(),
            record.fnd02().clone(),
            record.compatibility().clone(),
        )
    }

    fn authorization_for(
        predecessor_raw: u64,
        candidate: &ReconnectDurabilityRecordV1,
        current_scope: u64,
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
            ConnectionGeneration::new(7).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            None,
            CharacterLease::new(character(11)?, 9)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            Some(CharacterWorldEligibilityClaimV1::new(
                character(11)?,
                world(12)?,
            )),
            RuntimeScopeRefV1::channel(world(12)?, channel(13)?),
            ScopeOwnershipGeneration::new(current_scope)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?
        .with_control_loss_continuity(
            ControlLossEpochRefV1::new(3)?,
            candidate.continuity().original_grace_deadline(),
        )?;
        TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
            ACCOUNT,
            Some(&AccountPresenceClaimV1::new(
                candidate.identity().account_id(),
                candidate.identity().character_id(),
            )?),
            predecessor,
            candidate.identity().game_session_id(),
            snapshot,
            candidate,
        )
    }

    fn v2_request(
        candidate: ReconnectDurabilityRecordV1,
        predecessor_raw: u64,
        current_scope: u64,
    ) -> Result<ReconnectPrepareRequestV2, ReconnectDurabilityErrorV1> {
        let authorization = authorization_for(predecessor_raw, &candidate, current_scope)?;
        Ok(ReconnectDurabilityFlowV2::begin(candidate, Some(authorization)).1)
    }

    async fn seed_current_actor_anchor(
        database_url: &str,
        session_raw: u64,
        stored_scope: u64,
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
                encode($5, 'hex')::uuid, NULL, 3, $6, 7, 9, $7::text::numeric(20, 0), 7, 1\
             )",
        )
        .bind(uuid_v7(session_raw).as_slice())
        .bind(ACCOUNT)
        .bind(uuid_v7(11).as_slice())
        .bind(uuid_v7(12).as_slice())
        .bind(uuid_v7(13).as_slice())
        .bind(now + 120)
        .bind(stored_scope.to_string())
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

    async fn wait_for_lock_waiters(
        connection: &mut PgConnection,
        minimum: i64,
    ) -> Result<(), Box<dyn Error>> {
        for _ in 0..200 {
            let waiters: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM pg_stat_activity \
                 WHERE datname = current_database() \
                   AND pid <> pg_backend_pid() AND wait_event_type = 'Lock'",
            )
            .fetch_one(&mut *connection)
            .await?;
            if waiters >= minimum {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Err(format!("expected at least {minimum} PostgreSQL lock waiters").into())
    }

    fn map_legacy_prepare(
        disposition: ReconnectPrepareDispositionV1,
    ) -> ReconnectPrepareDispositionV2 {
        match disposition {
            ReconnectPrepareDispositionV1::Prepared => ReconnectPrepareDispositionV2::Prepared,
            ReconnectPrepareDispositionV1::ExistingPrepared => {
                ReconnectPrepareDispositionV2::ExistingPrepared
            }
            ReconnectPrepareDispositionV1::RejectedTransportRefCollision => {
                ReconnectPrepareDispositionV2::RejectedTransportRefCollision
            }
            ReconnectPrepareDispositionV1::RejectedConcurrentPrepared => {
                ReconnectPrepareDispositionV2::RejectedConcurrentPrepared
            }
            ReconnectPrepareDispositionV1::RejectedStaleAuthority => {
                ReconnectPrepareDispositionV2::RejectedStaleAuthority
            }
            ReconnectPrepareDispositionV1::AttemptCapacityExceeded => {
                ReconnectPrepareDispositionV2::AttemptCapacityExceeded
            }
            ReconnectPrepareDispositionV1::ExistingTerminal => {
                ReconnectPrepareDispositionV2::ExistingTerminal {
                    disposition: ReconnectDurableTerminalDispositionV1::StaleAuthority,
                }
            }
            ReconnectPrepareDispositionV1::Unavailable => {
                ReconnectPrepareDispositionV2::Unavailable
            }
            ReconnectPrepareDispositionV1::Ambiguous => ReconnectPrepareDispositionV2::Ambiguous,
            ReconnectPrepareDispositionV1::IdempotencyConflict => {
                ReconnectPrepareDispositionV2::IdempotencyConflict
            }
        }
    }

    fn failed_closed_prepare(
        result: &Result<ReconnectPrepareDispositionV2, DurabilityError>,
    ) -> bool {
        matches!(
            result,
            Ok(ReconnectPrepareDispositionV2::RejectedStaleAuthority)
                | Ok(ReconnectPrepareDispositionV2::IdempotencyConflict)
                | Err(DurabilityError::InvalidStoredState)
        )
    }

    #[allow(dead_code, async_fn_in_trait)]
    trait LegacyV2JournalFallback {
        async fn prepare_v2(
            &self,
            request: &ReconnectPrepareRequestV2,
        ) -> Result<ReconnectPrepareDispositionV2, DurabilityError>;

        async fn reconcile_v2(
            &self,
            request: &ReconnectPrepareRequestV2,
        ) -> Result<ReconnectDurableReconciliationSnapshotV2, DurabilityError>;
    }

    impl LegacyV2JournalFallback for LegacyAdmissionReconnectJournal {
        async fn prepare_v2(
            &self,
            request: &ReconnectPrepareRequestV2,
        ) -> Result<ReconnectPrepareDispositionV2, DurabilityError> {
            let (_flow, legacy_request) =
                ReconnectDurabilityFlowV1::begin(request.record().clone());
            self.prepare(&legacy_request).await.map(map_legacy_prepare)
        }

        async fn reconcile_v2(
            &self,
            request: &ReconnectPrepareRequestV2,
        ) -> Result<ReconnectDurableReconciliationSnapshotV2, DurabilityError> {
            let record = request.record().clone();
            let (_flow, legacy_request) = ReconnectDurabilityFlowV1::begin(record.clone());
            let legacy = self.reconcile(&legacy_request).await?;
            let outcome = if legacy
                == ReconnectDurableReconciliationSnapshotV1::prepared(record.clone())
            {
                ReconnectDurableOutcomeV2::Prepared
            } else if legacy == ReconnectDurableReconciliationSnapshotV1::committed(record.clone())
            {
                ReconnectDurableOutcomeV2::Committed {
                    current_generation: record.connection().candidate(),
                    current_transport_ref: record.connection().transport_ref(),
                }
            } else if legacy == ReconnectDurableReconciliationSnapshotV1::terminal(record.clone()) {
                ReconnectDurableOutcomeV2::Terminal {
                    disposition: ReconnectDurableTerminalDispositionV1::StaleAuthority,
                }
            } else {
                return Err(DurabilityError::InvalidStoredState);
            };
            Ok(ReconnectDurableReconciliationSnapshotV2::new(
                record, outcome,
            ))
        }
    }

    #[test]
    fn runtime_terminal_replacement_forward_syncs_scope_and_replays_exact_receipt() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) = migrated_database("forward_sync_receipt").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            seed_current_actor_anchor(&database_url, 10, 9, now).await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let candidate =
                record(20, 11, 1, 0xa1, 3, 7, 10, now).map_err(|_| "candidate record")?;
            let request = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;

            assert_eq!(
                journal.prepare_v2(&request).await?,
                ReconnectPrepareDispositionV2::Prepared
            );
            let mut connection = PgConnection::connect(&database_url).await?;
            let predecessor: (String, i16) = sqlx::query_as(
                "SELECT scope_ownership_generation::text, session_state \
                 FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(uuid_v7(10).as_slice())
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(predecessor, ("10".to_owned(), 3));
            let receipt_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game_durability_session_replacements \
                 WHERE character_id = encode($1, 'hex')::uuid \
                   AND predecessor_game_session_id = encode($2, 'hex')::uuid \
                   AND candidate_game_session_id = encode($3, 'hex')::uuid",
            )
            .bind(uuid_v7(11).as_slice())
            .bind(uuid_v7(10).as_slice())
            .bind(uuid_v7(20).as_slice())
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(receipt_count, 1);
            connection.close().await?;

            drop(journal);
            let recovered = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            assert_eq!(
                recovered.prepare_v2(&request).await?,
                ReconnectPrepareDispositionV2::ExistingPrepared
            );
            let conflicting = v2_request(request.record().clone(), 30, 10)
                .map_err(|_| "conflicting authorization")?;
            let conflict = recovered.prepare_v2(&conflicting).await;
            assert!(failed_closed_prepare(&conflict));
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn replacement_created_session_can_reconnect_in_later_epoch_without_reusing_replacement_authorization()
    -> TestResult {
        run_postgres_test(async {
            let (database, database_url) =
                migrated_database("replacement_candidate_later_epoch").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            seed_current_actor_anchor(&database_url, 10, 10, now).await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;

            let candidate =
                record(20, 11, 1, 0xa2, 3, 7, 10, now).map_err(|_| "candidate record")?;
            let authorization =
                authorization_for(10, &candidate, 10).map_err(|_| "replacement authorization")?;
            let (mut replacement_flow, replacement_request) =
                ReconnectDurabilityFlowV2::begin(candidate.clone(), Some(authorization));
            assert_eq!(
                journal.prepare_v2(&replacement_request).await?,
                ReconnectPrepareDispositionV2::Prepared
            );

            let mut budget =
                ReconnectAttemptBudgetV1::new(candidate.continuity().control_loss_epoch());
            budget
                .reserve(
                    candidate.identity().reconnect_attempt_ref(),
                    candidate.connection().transport_ref(),
                )
                .map_err(|_| "replacement attempt reservation")?;
            replacement_flow
                .accept_prepare_completion(
                    ReconnectPrepareCompletionV2::for_request(
                        &replacement_request,
                        ReconnectPrepareDispositionV2::Prepared,
                    ),
                    &mut budget,
                )
                .map_err(|_| "replacement prepare completion")?;
            let replacement_commit = replacement_flow
                .authorize_commit(
                    exact_current_authority(&candidate, now)
                        .map_err(|_| "current replacement authority")?,
                    now,
                )
                .map_err(|_| "replacement commit authorization")?;
            assert_eq!(
                journal.commit(&replacement_commit).await?,
                ReconnectCommitDispositionV1::Committed
            );

            let mut observer = PgConnection::connect(&database_url).await?;
            let receipt_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game_durability_session_replacements \
                 WHERE predecessor_game_session_id = encode($1, 'hex')::uuid \
                   AND candidate_game_session_id = encode($2, 'hex')::uuid",
            )
            .bind(uuid_v7(10).as_slice())
            .bind(uuid_v7(20).as_slice())
            .fetch_one(&mut observer)
            .await?;
            assert_eq!(receipt_count, 1);
            observer.close().await?;

            let later_epoch =
                record(20, 11, 2, 0xa3, 4, 8, 10, now).map_err(|_| "later epoch record")?;
            let (_, later_request) = ReconnectDurabilityFlowV1::begin(later_epoch.clone());
            assert_eq!(
                journal.prepare(&later_request).await?,
                ReconnectPrepareDispositionV1::Prepared
            );

            let replay = journal.prepare(&later_request).await;
            let later_v2_request = ReconnectDurabilityFlowV2::begin(later_epoch, None).1;
            let reconciliation = journal.reconcile_v2(&later_v2_request).await;
            let replay_ok = matches!(replay, Ok(ReconnectPrepareDispositionV1::ExistingPrepared));
            let reconciliation_ok = matches!(
                reconciliation,
                Ok(ref snapshot) if snapshot.outcome() == ReconnectDurableOutcomeV2::Prepared
            );
            assert!(
                replay_ok && reconciliation_ok,
                "later ordinary reconnect must replay and reconcile without historical replacement authorization; replay_ok={replay_ok}, reconciliation_ok={reconciliation_ok}"
            );

            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn replacement_created_session_can_replay_fresh_same_fence_attempt_after_collision()
    -> TestResult {
        run_postgres_test(async {
            let (database, database_url) =
                migrated_database("replacement_candidate_same_fence_retry").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            seed_current_actor_anchor(&database_url, 10, 10, now).await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;

            let reserved = record(50, 51, 1, 0xa2, 3, 7, 10, now)
                .map_err(|_| "transport reservation record")?;
            assert_eq!(
                journal
                    .prepare(&ReconnectDurabilityFlowV1::begin(reserved).1)
                    .await?,
                ReconnectPrepareDispositionV1::Prepared
            );

            let original =
                record(20, 11, 1, 0xa2, 3, 7, 10, now).map_err(|_| "candidate record")?;
            let authorization =
                authorization_for(10, &original, 10).map_err(|_| "replacement authorization")?;
            let original_request =
                ReconnectDurabilityFlowV2::begin(original, Some(authorization)).1;
            assert_eq!(
                journal.prepare_v2(&original_request).await?,
                ReconnectPrepareDispositionV2::RejectedTransportRefCollision
            );

            let fresh = record(20, 11, 2, 0xa3, 3, 7, 10, now).map_err(|_| "fresh attempt")?;
            let fresh_legacy_request = ReconnectDurabilityFlowV1::begin(fresh.clone()).1;
            assert_eq!(
                journal.prepare(&fresh_legacy_request).await?,
                ReconnectPrepareDispositionV1::Prepared
            );
            assert_eq!(
                journal.prepare(&fresh_legacy_request).await?,
                ReconnectPrepareDispositionV1::ExistingPrepared
            );

            let fresh_v2_request = ReconnectDurabilityFlowV2::begin(fresh, None).1;
            assert_eq!(
                journal.reconcile_v2(&fresh_v2_request).await?.outcome(),
                ReconnectDurableOutcomeV2::Prepared
            );

            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn runtime_terminal_replacement_rejects_consumed_fenced_continuity_before_mutation()
    -> TestResult {
        run_postgres_test(async {
            let (database, database_url) =
                migrated_database("replacement_consumed_fenced_continuity").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            seed_current_actor_anchor(&database_url, 10, 10, now).await?;

            let mut setup = PgConnection::connect(&database_url).await?;
            sqlx::query(
                "WITH activation AS (SELECT clock_timestamp() AS activated_at) \
                 UPDATE game_durability_control_loss_continuity AS continuity \
                 SET protection_entitlement_state = 2, protection_fenced_generation = 42, \
                     protection_activated_at = activation.activated_at, \
                     protection_expires_at = activation.activated_at + INTERVAL '4 seconds', \
                     protection_rearm_state = 2, \
                     protection_rearm_deadline = activation.activated_at + INTERVAL '8 seconds' \
                 FROM activation \
                 WHERE continuity.character_id = encode($1, 'hex')::uuid \
                   AND continuity.control_loss_epoch = 3",
            )
            .bind(uuid_v7(11).as_slice())
            .execute(&mut setup)
            .await?;
            setup.close().await?;

            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let candidate = record_with_protection(
                record(20, 11, 1, 0xaf, 3, 7, 10, now).map_err(|_| "candidate record")?,
                ProtectionEntitlementV1::fenced(42).map_err(|_| "fenced protection")?,
            )
            .map_err(|_| "candidate protection")?;
            let request = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
            let prepare_result = journal.prepare_v2(&request).await;

            let mut observer = PgConnection::connect(&database_url).await?;
            let predecessor_state: i16 = sqlx::query_scalar(
                "SELECT session_state FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(uuid_v7(10).as_slice())
            .fetch_one(&mut observer)
            .await?;
            let candidate_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(uuid_v7(20).as_slice())
            .fetch_one(&mut observer)
            .await?;
            let receipt_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game_durability_session_replacements \
                 WHERE predecessor_game_session_id = encode($1, 'hex')::uuid \
                   AND candidate_game_session_id = encode($2, 'hex')::uuid",
            )
            .bind(uuid_v7(10).as_slice())
            .bind(uuid_v7(20).as_slice())
            .fetch_one(&mut observer)
            .await?;
            observer.close().await?;
            drop(journal);
            database.cleanup().await?;

            assert!(matches!(
                prepare_result,
                Err(DurabilityError::InvalidStoredState)
            ));
            assert_eq!(predecessor_state, 1);
            assert_eq!(candidate_count, 0);
            assert_eq!(receipt_count, 0);
            Ok(())
        })
    }

    #[test]
    fn runtime_reconciliation_requires_exact_replacement_receipt_binding() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) = migrated_database("reconcile_receipt_binding").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            seed_current_actor_anchor(&database_url, 10, 9, now).await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let candidate =
                record(20, 11, 1, 0xa1, 3, 7, 10, now).map_err(|_| "candidate record")?;
            let request = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;

            assert_eq!(
                journal.prepare_v2(&request).await?,
                ReconnectPrepareDispositionV2::Prepared
            );
            drop(journal);

            let recovered = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            assert_eq!(
                recovered.reconcile_v2(&request).await?,
                ReconnectDurableReconciliationSnapshotV2::new(
                    request.record().clone(),
                    ReconnectDurableOutcomeV2::Prepared,
                )
            );

            let conflicting = v2_request(request.record().clone(), 30, 10)
                .map_err(|_| "conflicting authorization")?;
            assert!(matches!(
                recovered.reconcile_v2(&conflicting).await,
                Err(DurabilityError::InvalidStoredState)
            ));

            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn runtime_terminal_replacement_rejects_scope_ahead_without_candidate_mutation() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) = migrated_database("scope_ahead").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            seed_current_actor_anchor(&database_url, 10, 11, now).await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let candidate =
                record(20, 11, 1, 0xa2, 3, 7, 10, now).map_err(|_| "candidate record")?;
            let request = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
            let result = journal.prepare_v2(&request).await;
            assert!(failed_closed_prepare(&result));
            let mut connection = PgConnection::connect(&database_url).await?;
            let candidate_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(uuid_v7(20).as_slice())
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(candidate_count, 0);
            let receipt_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM game_durability_session_replacements")
                    .fetch_one(&mut connection)
                    .await?;
            assert_eq!(receipt_count, 0);
            connection.close().await?;
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn runtime_terminal_replacement_rejects_mismatched_predecessor_without_candidate_mutation()
    -> TestResult {
        run_postgres_test(async {
            let (database, database_url) = migrated_database("mismatched_predecessor").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            seed_current_actor_anchor(&database_url, 30, 10, now).await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let candidate =
                record(20, 11, 1, 0xa3, 3, 7, 10, now).map_err(|_| "candidate record")?;
            let request = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
            let result = journal.prepare_v2(&request).await;
            assert!(failed_closed_prepare(&result));
            let mut connection = PgConnection::connect(&database_url).await?;
            let candidate_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(uuid_v7(20).as_slice())
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(candidate_count, 0);
            connection.close().await?;
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn runtime_terminal_replacement_fences_prepared_predecessor_and_late_commit() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) = migrated_database("late_commit_fence").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let predecessor_record =
                record(10, 11, 1, 0xa4, 3, 7, 10, now).map_err(|_| "predecessor record")?;
            let (mut predecessor_flow, predecessor_prepare) =
                ReconnectDurabilityFlowV1::begin(predecessor_record);
            assert_eq!(
                journal.prepare(&predecessor_prepare).await?,
                ReconnectPrepareDispositionV1::Prepared
            );
            predecessor_flow
                .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &predecessor_prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(|_| "predecessor completion")?;
            let current = exact_current_authority(predecessor_prepare.record(), now)
                .map_err(|_| "predecessor current authority")?;
            let predecessor_commit = predecessor_flow
                .authorize_commit(current, now)
                .map_err(|_| "predecessor commit authorization")?;

            let candidate =
                record(20, 11, 2, 0xa5, 3, 7, 10, now).map_err(|_| "candidate record")?;
            let request = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
            assert_eq!(
                journal.prepare_v2(&request).await?,
                ReconnectPrepareDispositionV2::Prepared
            );

            let mut connection = PgConnection::connect(&database_url).await?;
            let predecessor_attempt_state: i16 = sqlx::query_scalar(
                "SELECT state FROM game_durability_reconnect_attempts \
                 WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
            )
            .bind(uuid_v7(10).as_slice())
            .bind(1_u64.to_be_bytes().as_slice())
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(predecessor_attempt_state, 4);
            let predecessor_state: i16 = sqlx::query_scalar(
                "SELECT session_state FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(uuid_v7(10).as_slice())
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(predecessor_state, 3);
            let prepared_ref: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT prepared_attempt_ref FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(uuid_v7(10).as_slice())
            .fetch_one(&mut connection)
            .await?;
            assert!(prepared_ref.is_none());
            connection.close().await?;
            let late_commit = journal.commit(&predecessor_commit).await;
            assert!(matches!(
                late_commit,
                Ok(ReconnectCommitDispositionV1::RejectedStaleAuthority)
                    | Ok(ReconnectCommitDispositionV1::ExistingTerminal)
                    | Err(DurabilityError::InvalidStoredState)
            ));
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn runtime_terminal_replacement_mid_transaction_failure_rolls_back_everything() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) = migrated_database("replacement_rollback").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            seed_current_actor_anchor(&database_url, 10, 9, now).await?;
            let mut connection = PgConnection::connect(&database_url).await?;
            connection
                .execute(sqlx::query(
                    "CREATE FUNCTION fail_after_replacement_receipt() RETURNS trigger \
                     LANGUAGE plpgsql AS $$ \
                     BEGIN \
                       IF EXISTS (SELECT 1 FROM game_durability_session_replacements) THEN \
                         RAISE EXCEPTION 'forced replacement rollback'; \
                       END IF; \
                       RETURN NEW; \
                     END $$",
                ))
                .await?;
            connection
                .execute(sqlx::query(
                    "CREATE TRIGGER fail_candidate_attempt_after_replacement \
                     BEFORE INSERT ON game_durability_reconnect_attempts \
                     FOR EACH ROW EXECUTE FUNCTION fail_after_replacement_receipt()",
                ))
                .await?;
            connection.close().await?;

            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let candidate =
                record(20, 11, 1, 0xa6, 3, 7, 10, now).map_err(|_| "candidate record")?;
            let request = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
            assert!(matches!(
                journal.prepare_v2(&request).await,
                Err(DurabilityError::Database(_))
            ));

            let mut connection = PgConnection::connect(&database_url).await?;
            let predecessor: (String, i16) = sqlx::query_as(
                "SELECT scope_ownership_generation::text, session_state \
                 FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(uuid_v7(10).as_slice())
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(predecessor, ("9".to_owned(), 1));
            let receipt_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM game_durability_session_replacements")
                    .fetch_one(&mut connection)
                    .await?;
            assert_eq!(receipt_count, 0);
            let candidate_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(uuid_v7(20).as_slice())
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(candidate_count, 0);
            connection.close().await?;
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn runtime_collision_replay_and_restart_reconciliation_preserve_collision_reason() -> TestResult
    {
        run_postgres_test(async {
            let (database, database_url) = migrated_database("typed_collision_restart").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let first = record(50, 51, 1, 0xb1, 3, 7, 10, now).map_err(|_| "first record")?;
            let (_first_flow, first_request) = ReconnectDurabilityFlowV1::begin(first);
            assert_eq!(
                journal.prepare(&first_request).await?,
                ReconnectPrepareDispositionV1::Prepared
            );

            let colliding =
                record(60, 61, 1, 0xb1, 3, 7, 10, now).map_err(|_| "collision record")?;
            let (_flow, collision_request) = ReconnectDurabilityFlowV2::begin(colliding, None);
            assert_eq!(
                journal.prepare_v2(&collision_request).await?,
                ReconnectPrepareDispositionV2::RejectedTransportRefCollision
            );
            drop(journal);

            let recovered = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            assert_eq!(
                recovered.prepare_v2(&collision_request).await?,
                ReconnectPrepareDispositionV2::ExistingTerminal {
                    disposition: ReconnectDurableTerminalDispositionV1::TransportRefCollision,
                }
            );
            assert_eq!(
                recovered.reconcile_v2(&collision_request).await?.outcome(),
                ReconnectDurableOutcomeV2::Terminal {
                    disposition: ReconnectDurableTerminalDispositionV1::TransportRefCollision,
                }
            );
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn runtime_restart_reconciliation_preserves_concurrent_and_stale_reasons() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) = migrated_database("typed_terminal_restart").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;

            let first = record(70, 71, 1, 0xc1, 3, 7, 10, now).map_err(|_| "first record")?;
            let (_first_flow, first_request) = ReconnectDurabilityFlowV1::begin(first);
            assert_eq!(
                journal.prepare(&first_request).await?,
                ReconnectPrepareDispositionV1::Prepared
            );
            let concurrent =
                record(70, 71, 2, 0xc2, 3, 7, 10, now).map_err(|_| "concurrent record")?;
            let (_flow, concurrent_request) = ReconnectDurabilityFlowV2::begin(concurrent, None);
            assert_eq!(
                journal.prepare_v2(&concurrent_request).await?,
                ReconnectPrepareDispositionV2::RejectedConcurrentPrepared
            );

            let stale = record(70, 71, 3, 0xc3, 4, 7, 10, now).map_err(|_| "stale record")?;
            let (_flow, stale_request) = ReconnectDurabilityFlowV2::begin(stale, None);
            assert_eq!(
                journal.prepare_v2(&stale_request).await?,
                ReconnectPrepareDispositionV2::RejectedStaleAuthority
            );
            drop(journal);

            let recovered = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            assert_eq!(
                recovered.reconcile_v2(&concurrent_request).await?.outcome(),
                ReconnectDurableOutcomeV2::Terminal {
                    disposition: ReconnectDurableTerminalDispositionV1::ConcurrentPrepared,
                }
            );
            assert_eq!(
                recovered.reconcile_v2(&stale_request).await?.outcome(),
                ReconnectDurableOutcomeV2::Terminal {
                    disposition: ReconnectDurableTerminalDispositionV1::StaleAuthority,
                }
            );
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn runtime_concurrent_terminal_replacement_has_exactly_one_candidate_winner() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) = migrated_database("replacement_race").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            seed_current_actor_anchor(&database_url, 10, 10, now).await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;

            let first_candidate =
                record(20, 11, 1, 0xd1, 3, 7, 10, now).map_err(|_| "first candidate")?;
            let second_candidate =
                record(21, 11, 1, 0xd2, 3, 7, 10, now).map_err(|_| "second candidate")?;
            let first_request = v2_request(first_candidate, 10, 10).map_err(|_| "first auth")?;
            let second_request = v2_request(second_candidate, 10, 10).map_err(|_| "second auth")?;
            let first_journal = journal.clone();
            let second_journal = journal.clone();
            let first_task =
                tokio::spawn(async move { first_journal.prepare_v2(&first_request).await });
            let second_task =
                tokio::spawn(async move { second_journal.prepare_v2(&second_request).await });
            let results = [first_task.await?, second_task.await?];
            let prepared = results
                .iter()
                .filter(|result| matches!(result, Ok(ReconnectPrepareDispositionV2::Prepared)))
                .count();
            let non_authoritative = results
                .iter()
                .filter(|result| {
                    matches!(
                        result,
                        Ok(ReconnectPrepareDispositionV2::RejectedStaleAuthority)
                            | Ok(ReconnectPrepareDispositionV2::IdempotencyConflict)
                            | Err(DurabilityError::InvalidStoredState)
                    )
                })
                .count();
            assert_eq!(prepared, 1);
            assert_eq!(non_authoritative, 1);

            let mut connection = PgConnection::connect(&database_url).await?;
            let live_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game_durability_reconnect_sessions \
                 WHERE character_id = encode($1, 'hex')::uuid AND session_state IN (1, 2)",
            )
            .bind(uuid_v7(11).as_slice())
            .fetch_one(&mut connection)
            .await?;
            assert_eq!(live_count, 1);
            connection.close().await?;
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn runtime_terminal_replacement_preserves_actor_epoch_attempt_budget() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) =
                migrated_database("replacement_preserves_attempt_budget").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            seed_current_actor_anchor(&database_url, 10, 10, now).await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;

            for attempt in 1_u64..=7 {
                let transport = u8::try_from(0xc0_u64 + attempt)?;
                let predecessor_record = record(10, 11, attempt, transport, 3, 7, 10, now)
                    .map_err(|_| "predecessor record")?;
                let (_, predecessor_request) = ReconnectDurabilityFlowV1::begin(predecessor_record);
                let expected = if attempt == 1 {
                    ReconnectPrepareDispositionV1::Prepared
                } else {
                    ReconnectPrepareDispositionV1::RejectedConcurrentPrepared
                };
                assert_eq!(journal.prepare(&predecessor_request).await?, expected);
            }

            let candidate =
                record(20, 11, 8, 0xd8, 3, 7, 10, now).map_err(|_| "candidate record")?;
            let replacement = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
            assert_eq!(
                journal.prepare_v2(&replacement).await?,
                ReconnectPrepareDispositionV2::Prepared
            );

            let mut observer = PgConnection::connect(&database_url).await?;
            let retained_count: i16 = sqlx::query_scalar(
                "SELECT attempt_count FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(uuid_v7(20).as_slice())
            .fetch_one(&mut observer)
            .await?;
            assert_eq!(retained_count, 8);
            observer.close().await?;

            let ninth_record =
                record(20, 11, 9, 0xd9, 3, 7, 10, now).map_err(|_| "ninth record")?;
            let (_, ninth_request) = ReconnectDurabilityFlowV1::begin(ninth_record);
            assert_eq!(
                journal.prepare(&ninth_request).await?,
                ReconnectPrepareDispositionV1::AttemptCapacityExceeded
            );

            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn runtime_delayed_predecessor_prepare_cannot_exceed_actor_epoch_attempt_budget() -> TestResult
    {
        run_postgres_test(async {
            let (database, database_url) =
                migrated_database("replacement_delayed_predecessor_attempt_budget").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            seed_current_actor_anchor(&database_url, 10, 10, now).await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;

            for attempt in 1_u64..=7 {
                let transport = u8::try_from(0xc0_u64 + attempt)?;
                let predecessor_record = record(10, 11, attempt, transport, 3, 7, 10, now)
                    .map_err(|_| "predecessor record")?;
                let (_, predecessor_request) = ReconnectDurabilityFlowV1::begin(predecessor_record);
                let expected = if attempt == 1 {
                    ReconnectPrepareDispositionV1::Prepared
                } else {
                    ReconnectPrepareDispositionV1::RejectedConcurrentPrepared
                };
                assert_eq!(journal.prepare(&predecessor_request).await?, expected);
            }

            let candidate =
                record(20, 11, 8, 0xd8, 3, 7, 10, now).map_err(|_| "candidate record")?;
            let replacement = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
            assert_eq!(
                journal.prepare_v2(&replacement).await?,
                ReconnectPrepareDispositionV2::Prepared
            );

            let delayed_predecessor =
                record(10, 11, 9, 0xd9, 3, 7, 10, now).map_err(|_| "delayed predecessor")?;
            let (_, delayed_request) = ReconnectDurabilityFlowV1::begin(delayed_predecessor);
            assert_eq!(
                journal.prepare(&delayed_request).await?,
                ReconnectPrepareDispositionV1::AttemptCapacityExceeded
            );

            let mut observer = PgConnection::connect(&database_url).await?;
            let retained: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game_durability_reconnect_attempts \
                 WHERE character_id = encode($1, 'hex')::uuid \
                   AND control_loss_epoch = $2::text::numeric(20, 0)",
            )
            .bind(uuid_v7(11).as_slice())
            .bind("3")
            .fetch_one(&mut observer)
            .await?;
            assert_eq!(retained, 8);
            observer.close().await?;

            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn runtime_terminal_replacement_rejects_exhausted_same_epoch_before_mutation() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) =
                migrated_database("replacement_rejects_exhausted_attempt_budget").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            seed_current_actor_anchor(&database_url, 10, 10, now).await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;

            for attempt in 1_u64..=8 {
                let transport = u8::try_from(0xc0_u64 + attempt)?;
                let predecessor_record = record(10, 11, attempt, transport, 3, 7, 10, now)
                    .map_err(|_| "predecessor record")?;
                let (_, predecessor_request) = ReconnectDurabilityFlowV1::begin(predecessor_record);
                let expected = if attempt == 1 {
                    ReconnectPrepareDispositionV1::Prepared
                } else {
                    ReconnectPrepareDispositionV1::RejectedConcurrentPrepared
                };
                assert_eq!(journal.prepare(&predecessor_request).await?, expected);
            }

            let candidate =
                record(20, 11, 9, 0xd9, 3, 7, 10, now).map_err(|_| "candidate record")?;
            let replacement = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
            assert_eq!(
                journal.prepare_v2(&replacement).await?,
                ReconnectPrepareDispositionV2::AttemptCapacityExceeded
            );

            let mut observer = PgConnection::connect(&database_url).await?;
            let predecessor_state: (i16, i16) = sqlx::query_as(
                "SELECT session_state, attempt_count FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(uuid_v7(10).as_slice())
            .fetch_one(&mut observer)
            .await?;
            assert_eq!(predecessor_state, (1, 8));
            let candidate_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(uuid_v7(20).as_slice())
            .fetch_one(&mut observer)
            .await?;
            assert_eq!(candidate_count, 0);
            let receipt_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM game_durability_session_replacements \
                 WHERE predecessor_game_session_id = encode($1, 'hex')::uuid \
                   AND candidate_game_session_id = encode($2, 'hex')::uuid",
            )
            .bind(uuid_v7(10).as_slice())
            .bind(uuid_v7(20).as_slice())
            .fetch_one(&mut observer)
            .await?;
            assert_eq!(receipt_count, 0);
            observer.close().await?;

            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn runtime_concurrent_identical_terminal_replacement_replays_exact_receipt() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) =
                migrated_database("replacement_exact_replay_race").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            seed_current_actor_anchor(&database_url, 10, 10, now).await?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let candidate =
                record(20, 11, 1, 0xe1, 3, 7, 10, now).map_err(|_| "candidate record")?;
            let request = v2_request(candidate, 10, 10).map_err(|_| "authorization")?;
            let mut blocker = PgConnection::connect(&database_url).await?;
            blocker.execute("BEGIN").await?;
            sqlx::query(
                "SELECT game_session_id FROM game_durability_reconnect_sessions \
                 WHERE game_session_id = encode($1, 'hex')::uuid FOR UPDATE",
            )
            .bind(uuid_v7(10).as_slice())
            .fetch_one(&mut blocker)
            .await?;
            let first_journal = journal.clone();
            let first_request = request.clone();
            let first = tokio::spawn(async move { first_journal.prepare_v2(&first_request).await });
            let second_journal = journal.clone();
            let second_request = request.clone();
            let second =
                tokio::spawn(async move { second_journal.prepare_v2(&second_request).await });
            let mut observer = PgConnection::connect(&database_url).await?;
            wait_for_lock_waiters(&mut observer, 2).await?;
            blocker.execute("COMMIT").await?;
            let results = [first.await?, second.await?];
            assert_eq!(
                results
                    .iter()
                    .filter(|result| matches!(result, Ok(ReconnectPrepareDispositionV2::Prepared)))
                    .count(),
                1
            );
            assert_eq!(
                results
                    .iter()
                    .filter(|result| {
                        matches!(result, Ok(ReconnectPrepareDispositionV2::ExistingPrepared))
                    })
                    .count(),
                1
            );
            observer.close().await?;
            blocker.close().await?;
            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }
    #[test]
    fn runtime_v2_reconciliation_racing_commit_uses_one_authoritative_snapshot() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) = migrated_database("reconcile_commit_snapshot").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let record = record(80, 81, 1, 0xe2, 3, 7, 10, now).map_err(|_| "record")?;
            let (mut flow, prepare_request) = ReconnectDurabilityFlowV1::begin(record.clone());
            assert_eq!(
                journal.prepare(&prepare_request).await?,
                ReconnectPrepareDispositionV1::Prepared
            );
            flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                &prepare_request,
                ReconnectPrepareDispositionV1::Prepared,
            ))
            .map_err(|_| "prepare completion")?;
            let current = exact_current_authority(&record, now).map_err(|_| "current authority")?;
            let commit_request = flow
                .authorize_commit(current, now)
                .map_err(|_| "commit authorization")?;
            let reconcile_request = ReconnectDurabilityFlowV2::begin(record.clone(), None).1;
            const LOCK_KEY: i64 = 25_220_260_829;
            let mut blocker = PgConnection::connect(&database_url).await?;
            sqlx::query("SELECT pg_advisory_lock($1)")
                .bind(LOCK_KEY)
                .execute(&mut blocker)
                .await?;
            let mut setup = PgConnection::connect(&database_url).await?;
            setup
                .execute(sqlx::query(
                    "CREATE FUNCTION block_reconnect_commit_update() RETURNS trigger \
                     LANGUAGE plpgsql AS $$ \
                     BEGIN \
                       PERFORM pg_advisory_xact_lock(25220260829); \
                       RETURN NEW; \
                     END $$",
                ))
                .await?;
            setup
                .execute(sqlx::query(
                    "CREATE TRIGGER block_reconnect_commit_update \
                     BEFORE UPDATE OF current_generation ON game_durability_reconnect_sessions \
                     FOR EACH ROW WHEN (NEW.current_generation IS DISTINCT FROM OLD.current_generation) \
                     EXECUTE FUNCTION block_reconnect_commit_update()",
                ))
                .await?;
            setup.close().await?;
            let commit_journal = journal.clone();
            let commit = tokio::spawn(async move { commit_journal.commit(&commit_request).await });
            let mut observer = PgConnection::connect(&database_url).await?;
            wait_for_lock_waiters(&mut observer, 1).await?;
            let reconcile_journal = journal.clone();
            let reconcile =
                tokio::spawn(
                    async move { reconcile_journal.reconcile_v2(&reconcile_request).await },
                );
            wait_for_lock_waiters(&mut observer, 2).await?;
            sqlx::query("SELECT pg_advisory_unlock($1)")
                .bind(LOCK_KEY)
                .execute(&mut blocker)
                .await?;
            assert_eq!(commit.await??, ReconnectCommitDispositionV1::Committed);
            assert_eq!(
                reconcile.await??.outcome(),
                ReconnectDurableOutcomeV2::Committed {
                    current_generation: record.connection().candidate(),
                    current_transport_ref: record.connection().transport_ref(),
                }
            );
            observer.close().await?;
            blocker.close().await?;
            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }
    #[test]
    fn runtime_v2_terminal_reconciliation_rejects_corrupt_attempt_mirrors() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) =
                migrated_database("terminal_attempt_mirror_corrupt").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let first = record(50, 51, 1, 0xe3, 3, 7, 10, now).map_err(|_| "first")?;
            let first_request = ReconnectDurabilityFlowV1::begin(first).1;
            assert_eq!(
                journal.prepare(&first_request).await?,
                ReconnectPrepareDispositionV1::Prepared
            );
            let terminal = record(60, 61, 1, 0xe3, 3, 7, 10, now).map_err(|_| "terminal")?;
            let request = ReconnectDurabilityFlowV2::begin(terminal, None).1;
            assert_eq!(
                journal.prepare_v2(&request).await?,
                ReconnectPrepareDispositionV2::RejectedTransportRefCollision
            );
            let mut connection = PgConnection::connect(&database_url).await?;
            sqlx::query(
                "UPDATE game_durability_reconnect_attempts \
                 SET control_loss_epoch = control_loss_epoch + 1 \
                 WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
            )
            .bind(uuid_v7(60).as_slice())
            .bind(1_u64.to_be_bytes().as_slice())
            .execute(&mut connection)
            .await?;
            connection.close().await?;
            assert!(matches!(
                journal.reconcile_v2(&request).await,
                Err(DurabilityError::InvalidStoredState)
            ));
            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }
    #[test]
    fn runtime_v2_terminal_reconciliation_rejects_corrupt_session_binding() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) =
                migrated_database("terminal_session_binding_corrupt").await?;
            let now = unix_now().map_err(|_| "invalid clock")?;
            let journal = AdmissionReconnectJournal::connect_runtime(&database_url).await?;
            let first = record(50, 51, 1, 0xe4, 3, 7, 10, now).map_err(|_| "first")?;
            let first_request = ReconnectDurabilityFlowV1::begin(first).1;
            assert_eq!(
                journal.prepare(&first_request).await?,
                ReconnectPrepareDispositionV1::Prepared
            );
            let terminal = record(60, 61, 1, 0xe4, 3, 7, 10, now).map_err(|_| "terminal")?;
            let request = ReconnectDurabilityFlowV2::begin(terminal, None).1;
            assert_eq!(
                journal.prepare_v2(&request).await?,
                ReconnectPrepareDispositionV2::RejectedTransportRefCollision
            );
            let mut connection = PgConnection::connect(&database_url).await?;
            sqlx::query(
                "UPDATE game_durability_reconnect_sessions \
                 SET account_id = '123e4567-e89b-12d3-a456-426614174001'::uuid \
                 WHERE game_session_id = encode($1, 'hex')::uuid",
            )
            .bind(uuid_v7(60).as_slice())
            .execute(&mut connection)
            .await?;
            connection.close().await?;
            assert!(matches!(
                journal.reconcile_v2(&request).await,
                Err(DurabilityError::InvalidStoredState)
            ));
            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }
}
