//! PostgreSQL-backed, journal-only reconnect durability boundary.
//!
//! This module deliberately owns persistence/classification only. Foundation
//! constructs and revalidates reconnect authority; the runtime must submit the
//! resulting request asynchronously and consume its completion as new input.

pub mod admission_authority_guards;
mod admission_journal;
pub mod character_authority;
pub mod character_authority_audit;
mod db;
pub mod fresh_admission;
pub mod fresh_admission_composition;
pub mod native_admission_source;
pub mod runtime_scope_assignment;
mod schema;

pub use admission_journal::AdmissionReconnectJournal;
pub use db::{DB_PASS_DEADLINE, DurabilityRoot, DurabilityRootConfig};
pub use schema::{MigrationExecutor, SchemaCompatibility};

#[cfg(test)]
mod fresh_admission_composition_linkage {
    use super::DurabilityRoot;
    use super::fresh_admission_composition::{
        COMPOSITION_SOURCE_AUTHORITY, FreshAdmissionComposition, FreshAdmissionSubject,
    };

    #[test]
    fn fresh_admission_composition_api_is_linked() {
        assert!(!COMPOSITION_SOURCE_AUTHORITY.is_empty());
        let _ = std::mem::size_of::<FreshAdmissionComposition>();
        let _ = std::mem::size_of::<FreshAdmissionSubject>();
        let _ = DurabilityRoot::publish_fresh_admission_sources;
        let _ = DurabilityRoot::compose_fresh_admission;
        let _ = DurabilityRoot::commit_composed_fresh_admission;
    }
}

#[cfg(test)]
mod native_admission_source_linkage {
    use super::DurabilityRoot;
    use super::native_admission_source::{
        DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
        PendingPublication, SourceObservation,
    };

    #[test]
    fn native_admission_source_api_is_linked() {
        let _ = std::mem::size_of::<FreshStoreProvenance>();
        let _ = std::mem::size_of::<DescriptorRegistration>();
        let _ = std::mem::size_of::<SourceObservation>();
        let _ = std::mem::size_of::<PendingPublication>();
        let _ = [
            NativeSourceOperation::ReadAccountSecurityV1,
            NativeSourceOperation::ReadFreshSigningTrustV1,
            NativeSourceOperation::ReadRecoveryAccountSecurityV2,
            NativeSourceOperation::ReadRecoverySigningTrustV2,
        ];
        let _ = NativeSourceOperation::parse("ReadAccountSecurityV1");
        let _ = NativeSourceSubject::account_security("01890f4c-3b2a-7cc2-8d11-9a321b7c0001");
        let _ = NativeSourceSubject::signing_trust(
            "urn:oteryn:platform:game-admission",
            "oteryn-pre-admission-v1",
            "fresh_admission",
            "key",
        );
        let _ = DurabilityRoot::initialize_native_admission_source;
        let _ = DurabilityRoot::claim_native_admission_source_custody;
        let _ = DurabilityRoot::register_native_admission_descriptor;
        let _ = DurabilityRoot::accept_native_source_observation;
        let _ = DurabilityRoot::checkpoint_native_source_publication;
        let _ = DurabilityRoot::clear_native_source_publication;
        let _ = DurabilityRoot::pending_native_source_publications;
    }
}

#[cfg(test)]
mod character_authority_linkage {
    use super::DurabilityRoot;
    use super::character_authority::{
        CharacterAuditDelivery, CharacterAuthorityError, CharacterAuthorityRecord,
        ReconciledCharacterAuthority,
    };

    #[test]
    fn character_authority_api_is_linked() {
        let _ = std::mem::size_of::<
            oteryn_game_server::character_bootstrap_intent::CharacterBootstrapIntentV1,
        >();
        let _ = std::mem::size_of::<CharacterAuditDelivery>();
        let _ = std::mem::size_of::<CharacterAuthorityError>();
        let _ = std::mem::size_of::<CharacterAuthorityRecord>();
        let _ = std::mem::size_of::<ReconciledCharacterAuthority<'_, '_>>();
        let _ = CharacterAuthorityError::Rejected;
        let _ = CharacterAuthorityError::Conflict;
        let _ = |error: CharacterAuthorityError| match error {
            CharacterAuthorityError::Unavailable(inner) => Some(inner),
            _ => None,
        };
        let _ = DurabilityRoot::bootstrap_character;
        let _ = DurabilityRoot::reconcile_character_bootstrap;
        let _ = oteryn_game_server::character_bootstrap_intent::read_authenticated_intent;
        let _ = oteryn_game_server::character_bootstrap_intent::CharacterInterpretationV1::new;
        let _ = DurabilityRoot::open_character_authority;
        let _ = DurabilityRoot::read_current_character;
        let _ = DurabilityRoot::admit_fresh_character_recovery;
        let _ = DurabilityRoot::reconcile_character_recovery;
        let _ = DurabilityRoot::pending_character_audit;
        let _ = DurabilityRoot::acknowledge_character_audit;
        let _ = DurabilityRoot::expire_character_audit;
        let _ = super::character_authority_audit::encode_bootstrap;
    }
}

#[cfg(test)]
mod runtime_scope_assignment_linkage {
    use super::DurabilityRoot;
    use super::runtime_scope_assignment::{
        AssignmentCommand, AssignmentError, AssignmentOutcome, AssignmentPredecessor,
        AssignmentReceipt, AssignmentRejection, AssignmentRequest, AssignmentState,
        BootstrapSecret, ControlActor, LaunchBinding, NodeRegistrationFact, OperationKey,
        ReconcileOutcome, RegistrationError, RuntimeScopeAssignment, RuntimeScopeAssignmentWriter,
    };

    #[test]
    fn runtime_scope_assignment_api_is_linked() {
        let _ = std::mem::size_of::<AssignmentCommand>();
        let _ = std::mem::size_of::<AssignmentError>();
        let _ = std::mem::size_of::<AssignmentOutcome>();
        let _ = std::mem::size_of::<AssignmentPredecessor>();
        let _ = std::mem::size_of::<AssignmentReceipt>();
        let _ = std::mem::size_of::<AssignmentRejection>();
        let _ = std::mem::size_of::<AssignmentRequest>();
        let _ = std::mem::size_of::<AssignmentState>();
        let _ = std::mem::size_of::<ReconcileOutcome>();
        let _ = std::mem::size_of::<RegistrationError>();
        let _ = std::mem::size_of::<RuntimeScopeAssignment>();
        let _ = BootstrapSecret::from_bytes([7; 32]);
        let _ = LaunchBinding::new("launch-1").map(|launch| launch.as_str().len());
        let _ = ControlActor::new("operator");
        let key = OperationKey::from_bytes([1; 32]);
        assert_eq!(OperationKey::from_text(&key.to_text()).ok(), Some(key));
        let _ = key.as_bytes();
        let _ = NodeRegistrationFact::new;
        let _ = NodeRegistrationFact::node_id;
        let _ = NodeRegistrationFact::registration_revision;
        let _ = super::runtime_scope_assignment::AssignmentReceipt::predecessor;
        let _ = DurabilityRoot::read_runtime_scope_predecessor;
        let _ = AssignmentRequest::encode;
        let _ = DurabilityRoot::issue_node_bootstrap_authorization;
        let _ = DurabilityRoot::register_node_incarnation;
        let _ = DurabilityRoot::require_current_node_registration;
        let _ = DurabilityRoot::revoke_node_registration;
        let _ = DurabilityRoot::read_runtime_scope_assignment;
        let _ = DurabilityRoot::publish_runtime_readiness;
        let _ = super::runtime_scope_assignment::NodeIncarnationProof::new;
        let _ = super::runtime_scope_assignment::NodeIncarnationProof::fact;
        let _ = AssignmentError::NotCurrentHolder;
        let _ = RuntimeScopeAssignmentWriter::open;
        let _ = RuntimeScopeAssignmentWriter::unreconciled;
        let _ = RuntimeScopeAssignmentWriter::submit;
        let _ = RuntimeScopeAssignmentWriter::reconcile;
        if let RegistrationError::Unavailable(inner) =
            RegistrationError::from(super::DurabilityError::Unavailable)
        {
            assert!(matches!(inner, super::DurabilityError::Unavailable));
        }
        if let AssignmentError::Unavailable(inner) =
            AssignmentError::from(super::DurabilityError::Unavailable)
        {
            assert!(matches!(inner, super::DurabilityError::Unavailable));
        }
        let uuid = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
        let (Ok(world), Ok(channel), Ok(node)) = (
            oteryn_game_server::foundation::WorldId::decode(&uuid),
            oteryn_game_server::foundation::ChannelId::decode(&uuid),
            oteryn_game_server::foundation::NodeId::decode(&uuid),
        ) else {
            return;
        };
        let scope = oteryn_game_server::foundation::RuntimeScopeRefV1::channel(world, channel);
        let target = NodeRegistrationFact::new(node, 1);
        let predecessor = AssignmentPredecessor {
            ownership_generation: 1,
            source_revision: 1,
            runtime_guard_publication_revision: Some(1),
        };
        let Ok(actor) = ControlActor::new("operator") else {
            return;
        };
        for command in [
            AssignmentCommand::Assign { scope, target },
            AssignmentCommand::Replace {
                scope,
                predecessor,
                target,
            },
            AssignmentCommand::Revoke { scope, predecessor },
        ] {
            let request = AssignmentRequest {
                operation_key: key,
                actor: actor.clone(),
                command,
            };
            assert!(request.encode().is_ok_and(|encoded| encoded.len() <= 1024));
        }
    }
}

use oteryn_game_server::foundation::{
    PendingCommandDispositionV1, ProtectionEntitlementV1, ReconnectDurabilityFlowV1,
    ReconnectDurabilityRecordV1, ReconnectDurableOutcomeV2,
    ReconnectDurableReconciliationSnapshotV1, ReconnectDurableReconciliationSnapshotV2,
    ReconnectDurableTerminalDispositionV1, ReconnectPrepareDispositionV1,
    ReconnectPrepareDispositionV2, ReconnectPrepareRequestV2, ReconnectProofV1, RuntimeScopeRefV1,
    TerminalGameSessionReplacementAuthorizationV1,
};
use serde_json::json;
use sqlx::{Postgres, Row, Transaction};
use std::fmt::{self, Display, Formatter};

// Accepted fixed first-slice registry345 at c9890968ce4c71165bdd9cd1d6938f9af75eaa00.
pub const MAX_FRESH_OPERATION_BYTES: usize = 65_536;
pub const MAX_ADMISSION_GUARD_BYTES: usize = 8_192;
pub const MAX_ADMISSION_ROW_BYTES: i64 = 131_072;

const V2_PREPARED: i16 = 1;
const V2_COLLISION_TERMINAL: i16 = 2;
const V2_CONCURRENT_TERMINAL: i16 = 3;
const V2_STALE_TERMINAL: i16 = 4;
const V2_COMMITTED: i16 = 5;
const V2_RECONNECTABLE: i16 = 1;
const V2_TERMINAL_SESSION: i16 = 3;
const V2_CHANNEL_SCOPE: i16 = 1;
const V2_INSTANCE_SCOPE: i16 = 2;
const V2_PENDING_ORIGINAL: i16 = 1;
const V2_TERMINAL_OUTCOME_RETAINED: i16 = 2;
const V2_PROTECTION_UNUSED: i16 = 1;
const V2_PROTECTION_FENCED: i16 = 2;
const V2_PROTECTION_REARM_READY: i16 = 1;
const V2_PROTECTION_REARM_PENDING: i16 = 2;
type V2ScopeStorage = (i16, Vec<u8>, Option<Vec<u8>>, Option<Vec<u8>>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlxFailureClass {
    Configuration,
    Database,
    Transport,
    Tls,
    Protocol,
    Data,
    Pool,
    Runtime,
    Migration,
    Transaction,
    Other,
}

impl SqlxFailureClass {
    fn classify(error: &sqlx::Error) -> Self {
        match error {
            sqlx::Error::Configuration(_) | sqlx::Error::InvalidArgument(_) => Self::Configuration,
            sqlx::Error::Database(_) => Self::Database,
            sqlx::Error::Io(_) => Self::Transport,
            sqlx::Error::Tls(_) => Self::Tls,
            sqlx::Error::Protocol(_) => Self::Protocol,
            sqlx::Error::RowNotFound
            | sqlx::Error::TypeNotFound { .. }
            | sqlx::Error::ColumnIndexOutOfBounds { .. }
            | sqlx::Error::ColumnNotFound(_)
            | sqlx::Error::ColumnDecode { .. }
            | sqlx::Error::Encode(_)
            | sqlx::Error::Decode(_)
            | sqlx::Error::AnyDriverError(_) => Self::Data,
            sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed => Self::Pool,
            sqlx::Error::WorkerCrashed => Self::Runtime,
            sqlx::Error::Migrate(_) => Self::Migration,
            sqlx::Error::InvalidSavePointStatement | sqlx::Error::BeginFailed => Self::Transaction,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationFailureClass {
    Execute(SqlxFailureClass),
    Source,
    Version,
    Dirty,
    Unsupported,
    Other,
}

impl MigrationFailureClass {
    fn classify(error: &sqlx::migrate::MigrateError) -> Self {
        match error {
            sqlx::migrate::MigrateError::Execute(error)
            | sqlx::migrate::MigrateError::ExecuteMigration(error, _) => {
                Self::Execute(SqlxFailureClass::classify(error))
            }
            sqlx::migrate::MigrateError::Source(_) => Self::Source,
            sqlx::migrate::MigrateError::VersionMissing(_)
            | sqlx::migrate::MigrateError::VersionMismatch(_)
            | sqlx::migrate::MigrateError::VersionNotPresent(_)
            | sqlx::migrate::MigrateError::VersionTooOld(_, _)
            | sqlx::migrate::MigrateError::VersionTooNew(_, _) => Self::Version,
            sqlx::migrate::MigrateError::Dirty(_) => Self::Dirty,
            sqlx::migrate::MigrateError::ForceNotSupported
            | sqlx::migrate::MigrateError::CreateSchemasNotSupported(_)
            | sqlx::migrate::MigrateError::SkipNotSupported() => Self::Unsupported,
            _ => Self::Other,
        }
    }
}

#[derive(Debug)]
pub enum DurabilityError {
    AdmissionGameSessionLedgerExhausted,
    SessionUseAuthorization(oteryn_game_server::foundation::admission_authority_publication::GameSessionUseAuthorizationErrorV1),
    Unavailable,
    Database(SqlxFailureClass),
    Migration(MigrationFailureClass),
    SchemaIncompatible(SchemaCompatibility),
    InvalidConfiguration,
    RootUnavailable,
    RootPassDeadlineExceeded,
    CommitRejected,
    CommitOutcomeUnknown,
    RootTaskFailed,
    InvalidStoredState,
}

impl Display for DurabilityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::AdmissionGameSessionLedgerExhausted => {
                formatter.write_str("ADMISSION_GAMESESSION_LEDGER_EXHAUSTED")
            }
            Self::SessionUseAuthorization(error) => {
                write!(formatter, "session use authorization failed ({error:?})")
            }
            Self::Unavailable => formatter.write_str("durability work is unavailable"),
            Self::Database(class) => {
                write!(
                    formatter,
                    "PostgreSQL durability operation failed ({class:?})"
                )
            }
            Self::Migration(class) => {
                write!(formatter, "game migration operation failed ({class:?})")
            }
            Self::SchemaIncompatible(state) => {
                write!(
                    formatter,
                    "game durability schema is not runtime-compatible: {state:?}"
                )
            }
            Self::InvalidConfiguration => {
                formatter.write_str("game durability root configuration is invalid")
            }
            Self::RootUnavailable => formatter.write_str("game durability root is not ready"),
            Self::RootPassDeadlineExceeded => {
                formatter.write_str("game durability semantic pass deadline expired")
            }
            Self::CommitRejected => {
                formatter.write_str("PostgreSQL rejected durability transaction commit")
            }
            Self::CommitOutcomeUnknown => {
                formatter.write_str("game durability commit outcome is unknown")
            }
            Self::RootTaskFailed => formatter.write_str("game durability root task failed"),
            Self::InvalidStoredState => {
                formatter.write_str("durability journal contains invalid state")
            }
        }
    }
}

impl std::error::Error for DurabilityError {}

impl DurabilityError {
    pub(crate) fn from_commit_error(error: sqlx::Error) -> Self {
        let class = SqlxFailureClass::classify(&error);
        drop(error);
        if class == SqlxFailureClass::Database {
            Self::CommitRejected
        } else {
            Self::CommitOutcomeUnknown
        }
    }
}

impl From<sqlx::Error> for DurabilityError {
    fn from(error: sqlx::Error) -> Self {
        let class = SqlxFailureClass::classify(&error);
        drop(error);
        Self::Database(class)
    }
}

impl From<sqlx::migrate::MigrateError> for DurabilityError {
    fn from(error: sqlx::migrate::MigrateError) -> Self {
        let class = MigrationFailureClass::classify(&error);
        drop(error);
        Self::Migration(class)
    }
}

#[cfg(test)]
mod wp3_error_classification_tests {
    use super::{DurabilityError, SqlxFailureClass};
    use std::fmt;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Clone, Default)]
    struct Observations {
        drops: Arc<AtomicUsize>,
        formats: Arc<AtomicUsize>,
    }

    struct PayloadProbe(Observations);

    impl Drop for PayloadProbe {
        fn drop(&mut self) {
            self.0.drops.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl fmt::Display for PayloadProbe {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.0.formats.fetch_add(1, Ordering::SeqCst);
            formatter.write_str("WP3_TEST_UNTRUSTED_PAYLOAD")
        }
    }

    impl fmt::Debug for PayloadProbe {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(self, formatter)
        }
    }

    impl std::error::Error for PayloadProbe {}

    impl sqlx::error::DatabaseError for PayloadProbe {
        fn message(&self) -> &str {
            "redacted-test-database-error"
        }

        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }

        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }

        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }

        fn kind(&self) -> sqlx::error::ErrorKind {
            sqlx::error::ErrorKind::Other
        }
    }

    #[test]
    fn sqlx_payload_is_classified_then_destroyed_without_formatting() {
        let seen = Observations::default();
        let error = DurabilityError::from(sqlx::Error::Tls(Box::new(PayloadProbe(seen.clone()))));

        assert!(matches!(
            error,
            DurabilityError::Database(SqlxFailureClass::Tls)
        ));
        assert_eq!(seen.drops.load(Ordering::SeqCst), 1);
        assert_eq!(seen.formats.load(Ordering::SeqCst), 0);
        assert!(!format!("{error}").contains("WP3_TEST_UNTRUSTED_PAYLOAD"));
        assert!(!format!("{error:?}").contains("WP3_TEST_UNTRUSTED_PAYLOAD"));
        assert_eq!(seen.formats.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn database_commit_rejection_is_definitive_without_payload_retention() {
        let seen = Observations::default();
        let error = DurabilityError::from_commit_error(sqlx::Error::Database(Box::new(
            PayloadProbe(seen.clone()),
        )));

        assert!(matches!(error, DurabilityError::CommitRejected));
        assert_eq!(seen.drops.load(Ordering::SeqCst), 1);
        assert_eq!(seen.formats.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn non_database_commit_error_stays_ambiguous_without_payload_retention() {
        let seen = Observations::default();
        let error = DurabilityError::from_commit_error(sqlx::Error::Tls(Box::new(PayloadProbe(
            seen.clone(),
        ))));

        assert!(matches!(error, DurabilityError::CommitOutcomeUnknown));
        assert_eq!(seen.drops.load(Ordering::SeqCst), 1);
        assert_eq!(seen.formats.load(Ordering::SeqCst), 0);
    }
}

/// Versioned successor for the canonical terminal-GameSession replacement boundary.
///
/// The legacy journal remains the proven V1 implementation. This wrapper owns only
/// the new Foundation-authorized predecessor->candidate transaction and typed V2
/// replay/reconciliation surface; ordinary V1 behavior is delegated unchanged.
#[derive(Clone)]
pub struct AdmissionReconnectJournalV2 {
    legacy: AdmissionReconnectJournal,
}

impl AdmissionReconnectJournalV2 {
    #[cfg(test)]
    pub async fn connect_runtime(database_url: &str) -> Result<Self, DurabilityError> {
        Ok(Self {
            legacy: AdmissionReconnectJournal::connect_runtime(database_url).await?,
        })
    }

    #[must_use]
    pub fn from_root(root: DurabilityRoot) -> Self {
        Self {
            legacy: AdmissionReconnectJournal::from_root(root),
        }
    }

    #[must_use]
    #[allow(dead_code)]
    pub(crate) const fn legacy(&self) -> &AdmissionReconnectJournal {
        &self.legacy
    }

    pub async fn prepare(
        &self,
        request: &ReconnectPrepareRequestV2,
    ) -> Result<ReconnectPrepareDispositionV2, DurabilityError> {
        self.prepare_owned(request, None).await
    }

    pub async fn prepare_with_claims(
        &self,
        request: &ReconnectPrepareRequestV2,
        transition: &oteryn_game_server::foundation::admission_authority_publication::TerminalReplacementClaimTransitionV1,
        membership: oteryn_game_server::foundation::admission_authority_publication::GameSessionUseRequestV1,
    ) -> Result<ReconnectPrepareDispositionV2, DurabilityError> {
        fresh_admission::encode_lifecycle(transition.evidence())?;
        self.prepare_owned(request, Some((transition.clone(), membership)))
            .await
    }

    async fn prepare_owned(
        &self,
        request: &ReconnectPrepareRequestV2,
        claims: Option<(oteryn_game_server::foundation::admission_authority_publication::TerminalReplacementClaimTransitionV1, oteryn_game_server::foundation::admission_authority_publication::GameSessionUseRequestV1)>,
    ) -> Result<ReconnectPrepareDispositionV2, DurabilityError> {
        let Some(authorization) = request.terminal_replacement() else {
            if claims.is_some() {
                return Err(DurabilityError::Unavailable);
            }
            return self.prepare_legacy_typed(request).await;
        };
        if !replacement_authorization_matches_record(authorization, request.record()) {
            return Err(DurabilityError::InvalidStoredState);
        }

        if let Some(issued) = self.legacy.try_issue_root_pass()? {
            let journal = self.clone();
            let request = request.clone();
            return db::await_root_task(self.legacy.spawn_root_task(async move {
                journal
                    .prepare_replacement_internal(request, Some(issued), claims)
                    .await
            })?)
            .await;
        }
        self.prepare_replacement_internal(request.clone(), None, claims)
            .await
    }

    async fn prepare_replacement_internal(
        &self,
        request: ReconnectPrepareRequestV2,
        issued: Option<db::IssuedSemanticPass>,
        claims: Option<(oteryn_game_server::foundation::admission_authority_publication::TerminalReplacementClaimTransitionV1, oteryn_game_server::foundation::admission_authority_publication::GameSessionUseRequestV1)>,
    ) -> Result<ReconnectPrepareDispositionV2, DurabilityError> {
        enum ReplacementPassResult {
            Disposition(ReconnectPrepareDispositionV2),
            ReplayReceipt,
        }

        let pass_request = request.clone();
        let claim_store = fresh_admission::FreshAdmissionStore::from_journal(&self.legacy);
        let pass_result = self
            .legacy
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let authorization = pass_request
                        .terminal_replacement()
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    let record = pass_request.record();
        let candidate_session_id = record.identity().game_session_id().as_bytes().to_vec();
        let character_id = record.identity().character_id().as_bytes().to_vec();
                    let mut transaction =
                        admission_journal::begin_pass_transaction(holder, deadline).await?;
                    db::lock_admission_domain(&mut transaction, record).await?;
                    if fresh_admission::has_owning_loss_receipt(
                        &mut transaction,
                        authorization.predecessor_game_session_id().as_bytes().as_slice(),
                        authorization.predecessor_control_loss_epoch().get(),
                    )
                    .await?
                    {
                        return Ok(ReplacementPassResult::Disposition(
                            ReconnectPrepareDispositionV2::Unavailable,
                        ));
                    }

        let claim_replay = if let Some((transition, _)) = &claims {
            claim_store.replacement_claim_replay(&mut transaction, transition).await?.is_some()
        } else { false };
        let claim_bearing: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id = encode($1,'hex')::uuid AND (fresh_replay_key IS NOT NULL OR initial_fresh_replay_key IS NOT NULL))")
            .bind(authorization.predecessor_game_session_id().as_bytes().as_slice()).fetch_one(&mut *transaction).await?;
        if claim_bearing && claims.is_none() { return Err(DurabilityError::Unavailable); }
        let candidate_exists =
            candidate_session_exists(&mut transaction, candidate_session_id.as_slice()).await?;
        if candidate_exists {
            if claims.is_some() && !claim_replay { return Err(DurabilityError::InvalidStoredState); }
            if !replacement_receipt_matches(&mut transaction, authorization, record).await? {
                return Err(DurabilityError::InvalidStoredState);
            }
            admission_journal::commit_pass_transaction(transaction, deadline).await?;
            return Ok(ReplacementPassResult::ReplayReceipt);
        }

        if replacement_receipt_for_candidate_exists(
            &mut transaction,
            character_id.as_slice(),
            candidate_session_id.as_slice(),
        )
        .await?
        {
            return Err(DurabilityError::InvalidStoredState);
        }

        let predecessor = sqlx::query(
            "SELECT uuid_send(game_session_id) AS game_session_id, \
                    account_id::text AS account_id, uuid_send(character_id) AS character_id, \
                    uuid_send(world_id) AS world_id, current_generation::text AS current_generation, \
                    control_loss_epoch::text AS control_loss_epoch, original_grace_deadline, \
                    character_lease_generation::text AS character_lease_generation, \
                    scope_ownership_generation::text AS scope_ownership_generation, \
                    prepared_attempt_ref \
             FROM game_durability_reconnect_sessions \
             WHERE character_id = encode($1, 'hex')::uuid \
               AND (session_state IN (1, 2) OR ($2 AND session_state = 3 AND game_session_id = encode($3,'hex')::uuid)) FOR UPDATE",
        )
        .bind(character_id.as_slice())
        .bind(claims.is_some())
        .bind(authorization.predecessor_game_session_id().as_bytes().as_slice())
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(predecessor) = predecessor else {
            if candidate_session_exists(&mut transaction, candidate_session_id.as_slice()).await?
                && replacement_receipt_matches(&mut transaction, authorization, record).await?
            {
                admission_journal::commit_pass_transaction(transaction, deadline).await?;
                return Ok(ReplacementPassResult::ReplayReceipt);
            }
            return Err(DurabilityError::InvalidStoredState);
        };
        if !replacement_predecessor_row_matches(&predecessor, authorization)? {
            return Err(DurabilityError::InvalidStoredState);
        }

        let stored_scope = predecessor
            .try_get::<String, _>("scope_ownership_generation")?
            .parse::<u64>()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let authorized_scope = authorization
            .predecessor_current_scope_ownership_generation()
            .get();
        if stored_scope > authorized_scope {
            return Err(DurabilityError::InvalidStoredState);
        }

        let retained_attempt_count =
            retained_actor_epoch_attempt_count_v2(&mut transaction, record).await?;
        if retained_attempt_count >= admission_journal::MAX_ATTEMPTS_PER_EPOCH {
            return Ok(ReplacementPassResult::Disposition(
                ReconnectPrepareDispositionV2::AttemptCapacityExceeded,
            ));
        }
        let membership_revision = fresh_admission::unused_session_revision(&mut transaction, record.identity().character_id(), record.identity().game_session_id()).await?;
        ensure_precommit_continuity_v2(&mut transaction, record, authorization).await?;

        if let Some(prepared_attempt_ref) =
            predecessor.try_get::<Option<Vec<u8>>, _>("prepared_attempt_ref")?
        {
            let terminalized = sqlx::query(
                "UPDATE game_durability_reconnect_attempts SET state = $3 \
                 WHERE game_session_id = encode($1, 'hex')::uuid \
                   AND reconnect_attempt_ref = $2 AND state = $4",
            )
            .bind(
                authorization
                    .predecessor_game_session_id()
                    .as_bytes()
                    .as_slice(),
            )
            .bind(prepared_attempt_ref.as_slice())
            .bind(V2_STALE_TERMINAL)
            .bind(V2_PREPARED)
            .execute(&mut *transaction)
            .await?;
            if terminalized.rows_affected() != 1 {
                return Err(DurabilityError::InvalidStoredState);
            }
        }

        sqlx::query(
            "UPDATE game_durability_reconnect_attempts SET state = $2 \
             WHERE game_session_id = encode($1, 'hex')::uuid AND state = $3",
        )
        .bind(
            authorization
                .predecessor_game_session_id()
                .as_bytes()
                .as_slice(),
        )
        .bind(V2_STALE_TERMINAL)
        .bind(V2_COMMITTED)
        .execute(&mut *transaction)
        .await?;

        let terminalized_session = sqlx::query(
            "UPDATE game_durability_reconnect_sessions \
             SET scope_ownership_generation = $2::text::numeric(20, 0), \
                 current_transport_ref = NULL, session_state = $3, prepared_attempt_ref = NULL \
             WHERE game_session_id = encode($1, 'hex')::uuid \
               AND current_generation = $4::text::numeric(20, 0) \
               AND character_lease_generation = $5::text::numeric(20, 0) \
               AND scope_ownership_generation <= $2::text::numeric(20, 0) \
               AND (session_state IN (1, 2) OR ($6 AND session_state = 3))",
        )
        .bind(
            authorization
                .predecessor_game_session_id()
                .as_bytes()
                .as_slice(),
        )
        .bind(authorized_scope.to_string())
        .bind(V2_TERMINAL_SESSION)
        .bind(
            authorization
                .predecessor_connection_generation()
                .get()
                .to_string(),
        )
        .bind(
            authorization
                .predecessor_character_lease_generation()
                .to_string(),
        )
        .bind(claims.is_some())
        .execute(&mut *transaction)
        .await?;
        if terminalized_session.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }

        let prepared_claims = if let Some((transition, membership)) = &claims {
            Some(claim_store.prepare_replacement_claims(&mut transaction, transition, *membership, record).await?)
        } else { None };
        fresh_admission::commit_session_use(&mut transaction, record.identity().character_id(), record.identity().game_session_id(), record.connection().transport_ref().to_bytes(), membership_revision).await?;
        let receipt = sqlx::query(
            "INSERT INTO game_durability_session_replacements (\
                character_id, predecessor_game_session_id, candidate_game_session_id, \
                candidate_reconnect_attempt_ref, \
                predecessor_connection_generation, predecessor_character_lease_generation, \
                predecessor_scope_ownership_generation\
             ) VALUES (\
                encode($1, 'hex')::uuid, encode($2, 'hex')::uuid, encode($3, 'hex')::uuid, \
                $4, $5::text::numeric(20, 0), $6::text::numeric(20, 0), \
                $7::text::numeric(20, 0)\
             ) ON CONFLICT DO NOTHING",
        )
        .bind(character_id.as_slice())
        .bind(
            authorization
                .predecessor_game_session_id()
                .as_bytes()
                .as_slice(),
        )
        .bind(candidate_session_id.as_slice())
        .bind(
            record
                .identity()
                .reconnect_attempt_ref()
                .to_be_bytes()
                .as_slice(),
        )
        .bind(
            authorization
                .predecessor_connection_generation()
                .get()
                .to_string(),
        )
        .bind(
            authorization
                .predecessor_character_lease_generation()
                .to_string(),
        )
        .bind(authorized_scope.to_string())
        .execute(&mut *transaction)
        .await?;
        if receipt.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }

        insert_candidate_session_v2(&mut transaction, record, retained_attempt_count).await?;
        if claims.is_some() {
            sqlx::query("UPDATE game_durability_reconnect_sessions SET initial_fresh_replay_key = (SELECT COALESCE(fresh_replay_key,initial_fresh_replay_key) FROM game_durability_reconnect_sessions WHERE game_session_id = encode($1,'hex')::uuid) WHERE game_session_id = encode($2,'hex')::uuid")
                .bind(authorization.predecessor_game_session_id().as_bytes().as_slice()).bind(record.identity().game_session_id().as_bytes().as_slice()).execute(&mut *transaction).await?;
        }

        let disposition =
            prepare_new_candidate_attempt_v2(&mut transaction, record, retained_attempt_count, prepared_claims.as_ref().map(|prepared| prepared.decided_at))
                .await?;
                    if let (Some((transition, _)), Some(prepared)) = (&claims, &prepared_claims) {
                        claim_store.persist_replacement_claims(&mut transaction, transition, prepared).await?;
                    }
                    admission_journal::commit_pass_transaction(transaction, deadline).await?;
                    Ok(ReplacementPassResult::Disposition(disposition))
                })
            })
            .await?;

        match pass_result {
            ReplacementPassResult::Disposition(disposition) => Ok(disposition),
            ReplacementPassResult::ReplayReceipt => {
                self.prepare_legacy_typed_receipt_authorized(&request).await
            }
        }
    }

    pub async fn reconcile(
        &self,
        request: &ReconnectPrepareRequestV2,
    ) -> Result<ReconnectDurableReconciliationSnapshotV2, DurabilityError> {
        if let Some(issued) = self.legacy.try_issue_root_pass()? {
            let journal = self.clone();
            let request = request.clone();
            return db::await_root_task(self.legacy.spawn_root_task(async move {
                journal.reconcile_internal(request, Some(issued)).await
            })?)
            .await;
        }
        self.reconcile_internal(request.clone(), None).await
    }

    async fn reconcile_internal(
        &self,
        request: ReconnectPrepareRequestV2,
        issued: Option<db::IssuedSemanticPass>,
    ) -> Result<ReconnectDurableReconciliationSnapshotV2, DurabilityError> {
        self.legacy
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let record = request.record();
                    let mut transaction =
                        admission_journal::begin_pass_transaction(holder, deadline).await?;
                    db::lock_admission_domain(&mut transaction, record).await?;
                    let (loss_session, loss_epoch) = match request.terminal_replacement() {
                        Some(authorization) => (
                            authorization.predecessor_game_session_id(),
                            authorization.predecessor_control_loss_epoch(),
                        ),
                        None => (
                            record.identity().game_session_id(),
                            record.continuity().control_loss_epoch(),
                        ),
                    };
                    if fresh_admission::has_owning_loss_receipt(
                        &mut transaction,
                        loss_session.as_bytes().as_slice(),
                        loss_epoch.get(),
                    )
                    .await?
                    {
                        return Err(DurabilityError::Unavailable);
                    }
                    if let Some(authorization) = request.terminal_replacement()
                        && (!replacement_authorization_matches_record(authorization, record)
                            || !replacement_receipt_matches(
                                &mut transaction,
                                authorization,
                                record,
                            )
                            .await?)
                    {
                        return Err(DurabilityError::InvalidStoredState);
                    }

                    let (legacy, state) =
                        AdmissionReconnectJournal::reconcile_record_in_transaction(
                            &mut transaction,
                            record,
                        )
                        .await?;
                    if request.terminal_replacement().is_none()
                        && admission_journal::replacement_receipt_matches_record(
                            &mut transaction,
                            record,
                        )
                        .await?
                    {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let outcome = match state {
                        V2_PREPARED => {
                            if legacy
                                != ReconnectDurableReconciliationSnapshotV1::prepared(
                                    record.clone(),
                                )
                            {
                                return Err(DurabilityError::InvalidStoredState);
                            }
                            ReconnectDurableOutcomeV2::Prepared
                        }
                        V2_COMMITTED => {
                            if legacy
                                != ReconnectDurableReconciliationSnapshotV1::committed(
                                    record.clone(),
                                )
                            {
                                return Err(DurabilityError::InvalidStoredState);
                            }
                            ReconnectDurableOutcomeV2::Committed {
                                current_generation: record.connection().candidate(),
                                current_transport_ref: record.connection().transport_ref(),
                            }
                        }
                        V2_COLLISION_TERMINAL | V2_CONCURRENT_TERMINAL | V2_STALE_TERMINAL => {
                            if legacy
                                != ReconnectDurableReconciliationSnapshotV1::terminal(
                                    record.clone(),
                                )
                            {
                                return Err(DurabilityError::InvalidStoredState);
                            }
                            ReconnectDurableOutcomeV2::Terminal {
                                disposition: terminal_disposition_from_state(state)?,
                            }
                        }
                        _ => return Err(DurabilityError::InvalidStoredState),
                    };
                    admission_journal::commit_pass_transaction(transaction, deadline).await?;
                    Ok(ReconnectDurableReconciliationSnapshotV2::new(
                        record.clone(),
                        outcome,
                    ))
                })
            })
            .await
    }

    async fn prepare_legacy_typed(
        &self,
        request: &ReconnectPrepareRequestV2,
    ) -> Result<ReconnectPrepareDispositionV2, DurabilityError> {
        self.prepare_legacy_typed_internal(request, false).await
    }

    async fn prepare_legacy_typed_receipt_authorized(
        &self,
        request: &ReconnectPrepareRequestV2,
    ) -> Result<ReconnectPrepareDispositionV2, DurabilityError> {
        self.prepare_legacy_typed_internal(request, true).await
    }

    async fn prepare_legacy_typed_internal(
        &self,
        request: &ReconnectPrepareRequestV2,
        receipt_authorized: bool,
    ) -> Result<ReconnectPrepareDispositionV2, DurabilityError> {
        let record = request.record();
        let (_, legacy_request) = ReconnectDurabilityFlowV1::begin(record.clone());
        let disposition = if receipt_authorized {
            self.legacy
                .prepare_receipt_authorized(&legacy_request)
                .await?
        } else {
            self.legacy.prepare(&legacy_request).await?
        };
        match disposition {
            ReconnectPrepareDispositionV1::Prepared => Ok(ReconnectPrepareDispositionV2::Prepared),
            ReconnectPrepareDispositionV1::ExistingPrepared => {
                Ok(ReconnectPrepareDispositionV2::ExistingPrepared)
            }
            ReconnectPrepareDispositionV1::RejectedTransportRefCollision => {
                Ok(ReconnectPrepareDispositionV2::RejectedTransportRefCollision)
            }
            ReconnectPrepareDispositionV1::RejectedConcurrentPrepared => {
                Ok(ReconnectPrepareDispositionV2::RejectedConcurrentPrepared)
            }
            ReconnectPrepareDispositionV1::RejectedStaleAuthority => {
                Ok(ReconnectPrepareDispositionV2::RejectedStaleAuthority)
            }
            ReconnectPrepareDispositionV1::AttemptCapacityExceeded => {
                Ok(ReconnectPrepareDispositionV2::AttemptCapacityExceeded)
            }
            ReconnectPrepareDispositionV1::ExistingTerminal => {
                let state = self.terminal_state_for_record(record).await?;
                Ok(ReconnectPrepareDispositionV2::ExistingTerminal {
                    disposition: terminal_disposition_from_state(state)?,
                })
            }
            ReconnectPrepareDispositionV1::Unavailable => {
                Ok(ReconnectPrepareDispositionV2::Unavailable)
            }
            ReconnectPrepareDispositionV1::Ambiguous => {
                Ok(ReconnectPrepareDispositionV2::Ambiguous)
            }
            ReconnectPrepareDispositionV1::IdempotencyConflict => {
                Ok(ReconnectPrepareDispositionV2::IdempotencyConflict)
            }
        }
    }

    async fn terminal_state_for_record(
        &self,
        record: &ReconnectDurabilityRecordV1,
    ) -> Result<i16, DurabilityError> {
        let record = record.clone();
        if let Some(issued) = self.legacy.try_issue_root_pass()? {
            let journal = self.clone();
            return db::await_root_task(self.legacy.spawn_root_task(async move {
                journal
                    .terminal_state_for_record_internal(record, Some(issued))
                    .await
            })?)
            .await;
        }
        self.terminal_state_for_record_internal(record, None).await
    }

    async fn terminal_state_for_record_internal(
        &self,
        record: ReconnectDurabilityRecordV1,
        issued: Option<db::IssuedSemanticPass>,
    ) -> Result<i16, DurabilityError> {
        self.legacy
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let mut transaction =
                        admission_journal::begin_pass_transaction(holder, deadline).await?;
                    db::lock_admission_domain(&mut transaction, &record).await?;
                    if fresh_admission::has_owning_loss_receipt(
                        &mut transaction,
                        record.identity().game_session_id().as_bytes().as_slice(),
                        record.continuity().control_loss_epoch().get(),
                    )
                    .await?
                    {
                        return Err(DurabilityError::Unavailable);
                    }
                    let row = sqlx::query(
                        "SELECT state, record_json FROM game_durability_reconnect_attempts \
                         WHERE game_session_id = encode($1, 'hex')::uuid AND reconnect_attempt_ref = $2",
                    )
                    .bind(record.identity().game_session_id().as_bytes().as_slice())
                    .bind(
                        record
                            .identity()
                            .reconnect_attempt_ref()
                            .to_be_bytes()
                            .as_slice(),
                    )
                    .fetch_optional(&mut *transaction)
                    .await?;
                    let Some(row) = row else {
                        return Err(DurabilityError::InvalidStoredState);
                    };
                    let stored_record = row.try_get::<String, _>("record_json")?;
                    let stored_record: serde_json::Value = serde_json::from_str(&stored_record)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    if stored_record != encode_record_v2(&record) {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let state = row.try_get("state").map_err(DurabilityError::from)?;
                    admission_journal::commit_pass_transaction(transaction, deadline).await?;
                    Ok(state)
                })
            })
            .await
    }
}

fn replacement_authorization_matches_record(
    authorization: &TerminalGameSessionReplacementAuthorizationV1,
    record: &ReconnectDurabilityRecordV1,
) -> bool {
    let identity = record.identity();
    authorization.account_id() == identity.account_id()
        && authorization.character_id() == identity.character_id()
        && authorization.world_id() == identity.world_id()
        && authorization.candidate_game_session_id() == identity.game_session_id()
        && authorization.candidate_runtime_scope() == identity.runtime_scope()
        && authorization.predecessor_connection_generation() == record.connection().predecessor()
        && authorization.predecessor_character_lease_generation()
            == record.authority().character_lease_generation()
        && authorization.predecessor_current_scope_ownership_generation()
            == record.authority().scope_ownership_generation()
        && authorization.predecessor_control_loss_epoch()
            == record.continuity().control_loss_epoch()
        && authorization.predecessor_original_grace_deadline()
            == record.continuity().original_grace_deadline()
}

fn replacement_predecessor_row_matches(
    row: &sqlx::postgres::PgRow,
    authorization: &TerminalGameSessionReplacementAuthorizationV1,
) -> Result<bool, DurabilityError> {
    Ok(row.try_get::<Vec<u8>, _>("game_session_id")?.as_slice()
        == authorization
            .predecessor_game_session_id()
            .as_bytes()
            .as_slice()
        && row.try_get::<String, _>("account_id")? == authorization.account_id()
        && row.try_get::<Vec<u8>, _>("character_id")?.as_slice()
            == authorization.character_id().as_bytes().as_slice()
        && row.try_get::<Vec<u8>, _>("world_id")?.as_slice()
            == authorization.world_id().as_bytes().as_slice()
        && row.try_get::<String, _>("current_generation")?
            == authorization
                .predecessor_connection_generation()
                .get()
                .to_string()
        && row.try_get::<String, _>("character_lease_generation")?
            == authorization
                .predecessor_character_lease_generation()
                .to_string()
        && row.try_get::<String, _>("control_loss_epoch")?
            == authorization
                .predecessor_control_loss_epoch()
                .get()
                .to_string()
        && row.try_get::<i64, _>("original_grace_deadline")?
            == authorization.predecessor_original_grace_deadline())
}

async fn candidate_session_exists(
    transaction: &mut Transaction<'_, Postgres>,
    candidate_session_id: &[u8],
) -> Result<bool, DurabilityError> {
    sqlx::query_scalar(
        "SELECT EXISTS (\
            SELECT 1 FROM game_durability_reconnect_sessions \
            WHERE game_session_id = encode($1, 'hex')::uuid\
         )",
    )
    .bind(candidate_session_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(DurabilityError::from)
}

async fn replacement_receipt_for_candidate_exists(
    transaction: &mut Transaction<'_, Postgres>,
    character_id: &[u8],
    candidate_session_id: &[u8],
) -> Result<bool, DurabilityError> {
    sqlx::query_scalar(
        "SELECT EXISTS (\
            SELECT 1 FROM game_durability_session_replacements \
            WHERE character_id = encode($1, 'hex')::uuid \
              AND candidate_game_session_id = encode($2, 'hex')::uuid\
         )",
    )
    .bind(character_id)
    .bind(candidate_session_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(DurabilityError::from)
}

async fn replacement_receipt_matches(
    transaction: &mut Transaction<'_, Postgres>,
    authorization: &TerminalGameSessionReplacementAuthorizationV1,
    record: &ReconnectDurabilityRecordV1,
) -> Result<bool, DurabilityError> {
    let row = sqlx::query(
        "SELECT uuid_send(predecessor_game_session_id) AS predecessor_game_session_id, \
                predecessor_connection_generation::text AS predecessor_connection_generation, \
                predecessor_character_lease_generation::text AS predecessor_character_lease_generation, \
                predecessor_scope_ownership_generation::text AS predecessor_scope_ownership_generation, \
                candidate_reconnect_attempt_ref \
         FROM game_durability_session_replacements \
         WHERE character_id = encode($1, 'hex')::uuid \
           AND candidate_game_session_id = encode($2, 'hex')::uuid FOR SHARE",
    )
    .bind(authorization.character_id().as_bytes().as_slice())
    .bind(authorization.candidate_game_session_id().as_bytes().as_slice())
    .fetch_optional(&mut **transaction)
    .await?;
    let Some(row) = row else {
        return Ok(false);
    };
    Ok(row
        .try_get::<Vec<u8>, _>("predecessor_game_session_id")?
        .as_slice()
        == authorization
            .predecessor_game_session_id()
            .as_bytes()
            .as_slice()
        && row
            .try_get::<Vec<u8>, _>("candidate_reconnect_attempt_ref")?
            .as_slice()
            == record
                .identity()
                .reconnect_attempt_ref()
                .to_be_bytes()
                .as_slice()
        && row.try_get::<String, _>("predecessor_connection_generation")?
            == authorization
                .predecessor_connection_generation()
                .get()
                .to_string()
        && row.try_get::<String, _>("predecessor_character_lease_generation")?
            == authorization
                .predecessor_character_lease_generation()
                .to_string()
        && row.try_get::<String, _>("predecessor_scope_ownership_generation")?
            == authorization
                .predecessor_current_scope_ownership_generation()
                .get()
                .to_string())
}

async fn retained_actor_epoch_attempt_count_v2(
    transaction: &mut Transaction<'_, Postgres>,
    record: &ReconnectDurabilityRecordV1,
) -> Result<i16, DurabilityError> {
    admission_journal::lock_actor_epoch_attempt_budget(transaction, record)
        .await?
        .ok_or(DurabilityError::InvalidStoredState)
}

async fn insert_candidate_session_v2(
    transaction: &mut Transaction<'_, Postgres>,
    record: &ReconnectDurabilityRecordV1,
    retained_attempt_count: i16,
) -> Result<(), DurabilityError> {
    let identity = record.identity();
    let (scope_kind, scope_world_id, scope_channel_id, scope_instance_id) =
        scope_storage_v2(record);
    let inserted = sqlx::query(
        "INSERT INTO game_durability_reconnect_sessions (\
            game_session_id, account_id, character_id, world_id, runtime_scope_kind, \
            runtime_scope_world_id, runtime_scope_channel_id, runtime_scope_instance_id, \
            control_loss_epoch, original_grace_deadline, predecessor_generation, \
            character_lease_generation, scope_ownership_generation, current_generation, \
            attempt_count, session_state\
         ) VALUES (\
            encode($1, 'hex')::uuid, $2::text::uuid, encode($3, 'hex')::uuid, \
            encode($4, 'hex')::uuid, $5, encode($6, 'hex')::uuid, \
            encode($7, 'hex')::uuid, encode($8, 'hex')::uuid, \
            $9::text::numeric(20, 0), $10, $11::text::numeric(20, 0), \
            $12::text::numeric(20, 0), $13::text::numeric(20, 0), \
            $11::text::numeric(20, 0), $14, $15\
         )",
    )
    .bind(identity.game_session_id().as_bytes().as_slice())
    .bind(identity.account_id())
    .bind(identity.character_id().as_bytes().as_slice())
    .bind(identity.world_id().as_bytes().as_slice())
    .bind(scope_kind)
    .bind(scope_world_id.as_slice())
    .bind(scope_channel_id.as_deref())
    .bind(scope_instance_id.as_deref())
    .bind(record.continuity().control_loss_epoch().get().to_string())
    .bind(record.continuity().original_grace_deadline())
    .bind(record.connection().predecessor().get().to_string())
    .bind(record.authority().character_lease_generation().to_string())
    .bind(
        record
            .authority()
            .scope_ownership_generation()
            .get()
            .to_string(),
    )
    .bind(retained_attempt_count)
    .bind(V2_RECONNECTABLE)
    .execute(&mut **transaction)
    .await?;
    if inserted.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

async fn ensure_precommit_continuity_v2(
    transaction: &mut Transaction<'_, Postgres>,
    record: &ReconnectDurabilityRecordV1,
    authorization: &TerminalGameSessionReplacementAuthorizationV1,
) -> Result<(), DurabilityError> {
    let identity = record.identity();
    let (state, fenced_generation, rearm_state) = match record.continuity().protection_entitlement()
    {
        ProtectionEntitlementV1::Unused => (V2_PROTECTION_UNUSED, None, V2_PROTECTION_REARM_READY),
        ProtectionEntitlementV1::Fenced { generation } => (
            V2_PROTECTION_FENCED,
            Some(generation.to_string()),
            V2_PROTECTION_REARM_PENDING,
        ),
    };
    // A terminal replacement may inherit the exact same loss-epoch continuity row.
    // Validate it without changing entitlement/rearm timestamps, then rebind only the
    // GameSession context under the same predecessor->candidate transaction.
    let existing = sqlx::query(
        "SELECT account_id::text AS account_id, uuid_send(world_id) AS world_id, \
                uuid_send(context_game_session_id) AS context_game_session_id, \
                original_grace_deadline, protection_entitlement_state, \
                protection_fenced_generation::text AS protection_fenced_generation, \
                protection_activated_at IS NULL AS protection_activation_missing, \
                protection_expires_at IS NULL AS protection_expiry_missing, \
                CASE WHEN protection_activated_at IS NOT NULL AND protection_expires_at IS NOT NULL \
                     THEN EXTRACT(EPOCH FROM (protection_expires_at - protection_activated_at))::BIGINT \
                     ELSE NULL END AS protection_duration_seconds, \
                protection_rearm_state, \
                protection_rearm_deadline IS NULL AS protection_rearm_deadline_missing \
         FROM game_durability_control_loss_continuity \
         WHERE character_id = encode($1, 'hex')::uuid \
           AND control_loss_epoch = $2::text::numeric(20, 0) FOR UPDATE",
    )
    .bind(identity.character_id().as_bytes().as_slice())
    .bind(record.continuity().control_loss_epoch().get().to_string())
    .fetch_optional(&mut **transaction)
    .await?;
    let Some(existing) = existing else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let context: Vec<u8> = existing.try_get("context_game_session_id")?;
    let stored_fenced_generation: Option<String> =
        existing.try_get("protection_fenced_generation")?;
    let exact_continuity = existing.try_get::<String, _>("account_id")? == identity.account_id()
        && existing.try_get::<Vec<u8>, _>("world_id")?.as_slice()
            == identity.world_id().as_bytes().as_slice()
        && existing.try_get::<i64, _>("original_grace_deadline")?
            == record.continuity().original_grace_deadline()
        && existing.try_get::<i16, _>("protection_entitlement_state")? == state
        && stored_fenced_generation.as_deref() == fenced_generation.as_deref()
        && existing.try_get::<bool, _>("protection_activation_missing")?
        && existing.try_get::<bool, _>("protection_expiry_missing")?
        && existing
            .try_get::<Option<i64>, _>("protection_duration_seconds")?
            .is_none()
        && existing.try_get::<i16, _>("protection_rearm_state")? == rearm_state
        && existing.try_get::<bool, _>("protection_rearm_deadline_missing")?;
    if !exact_continuity {
        return Err(DurabilityError::InvalidStoredState);
    }

    let candidate_game_session_id = identity.game_session_id();
    let candidate_session_id = candidate_game_session_id.as_bytes();
    if context.as_slice() == candidate_session_id.as_slice() {
        return Ok(());
    }
    if context.as_slice()
        != authorization
            .predecessor_game_session_id()
            .as_bytes()
            .as_slice()
    {
        return Err(DurabilityError::InvalidStoredState);
    }

    let rebound = sqlx::query(
        "UPDATE game_durability_control_loss_continuity \
         SET context_game_session_id = encode($3, 'hex')::uuid \
         WHERE character_id = encode($1, 'hex')::uuid \
           AND control_loss_epoch = $2::text::numeric(20, 0) \
           AND context_game_session_id = encode($4, 'hex')::uuid",
    )
    .bind(identity.character_id().as_bytes().as_slice())
    .bind(record.continuity().control_loss_epoch().get().to_string())
    .bind(candidate_session_id.as_slice())
    .bind(
        authorization
            .predecessor_game_session_id()
            .as_bytes()
            .as_slice(),
    )
    .execute(&mut **transaction)
    .await?;
    if rebound.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

async fn prepare_new_candidate_attempt_v2(
    transaction: &mut Transaction<'_, Postgres>,
    record: &ReconnectDurabilityRecordV1,
    retained_attempt_count: i16,
    decided_at: Option<i64>,
) -> Result<ReconnectPrepareDispositionV2, DurabilityError> {
    if retained_attempt_count >= admission_journal::MAX_ATTEMPTS_PER_EPOCH {
        return Ok(ReconnectPrepareDispositionV2::AttemptCapacityExceeded);
    }
    let now = match decided_at {
        Some(now) => now,
        None => database_now_v2(transaction).await?,
    };
    if now > record.continuity().prepared_deadline() {
        insert_attempt_v2(transaction, record, V2_STALE_TERMINAL).await?;
        set_candidate_attempt_v2(transaction, record, None, retained_attempt_count).await?;
        return Ok(ReconnectPrepareDispositionV2::RejectedStaleAuthority);
    }

    let transport_ref = record.connection().transport_ref().to_bytes();
    let attempt_ref = record.identity().reconnect_attempt_ref().to_be_bytes();
    let reserved = sqlx::query(
        "INSERT INTO game_durability_transport_ref_reservations (\
            transport_ref, game_session_id, reconnect_attempt_ref\
         ) VALUES ($1, encode($2, 'hex')::uuid, $3) \
         ON CONFLICT (transport_ref) DO NOTHING",
    )
    .bind(transport_ref.as_slice())
    .bind(record.identity().game_session_id().as_bytes().as_slice())
    .bind(attempt_ref.as_slice())
    .execute(&mut **transaction)
    .await?;
    if reserved.rows_affected() == 0 {
        insert_attempt_v2(transaction, record, V2_COLLISION_TERMINAL).await?;
        set_candidate_attempt_v2(transaction, record, None, retained_attempt_count).await?;
        return Ok(ReconnectPrepareDispositionV2::RejectedTransportRefCollision);
    }

    insert_attempt_v2(transaction, record, V2_PREPARED).await?;
    set_candidate_attempt_v2(
        transaction,
        record,
        Some(attempt_ref.as_slice()),
        retained_attempt_count,
    )
    .await?;
    Ok(ReconnectPrepareDispositionV2::Prepared)
}

async fn set_candidate_attempt_v2(
    transaction: &mut Transaction<'_, Postgres>,
    record: &ReconnectDurabilityRecordV1,
    prepared_attempt_ref: Option<&[u8]>,
    retained_attempt_count: i16,
) -> Result<(), DurabilityError> {
    let updated = sqlx::query(
        "UPDATE game_durability_reconnect_sessions \
         SET attempt_count = attempt_count + 1, prepared_attempt_ref = $2 \
         WHERE game_session_id = encode($1, 'hex')::uuid \
           AND session_state = $3 AND attempt_count = $4",
    )
    .bind(record.identity().game_session_id().as_bytes().as_slice())
    .bind(prepared_attempt_ref)
    .bind(V2_RECONNECTABLE)
    .bind(retained_attempt_count)
    .execute(&mut **transaction)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

async fn insert_attempt_v2(
    transaction: &mut Transaction<'_, Postgres>,
    record: &ReconnectDurabilityRecordV1,
    state: i16,
) -> Result<(), DurabilityError> {
    let identity = record.identity();
    let (scope_kind, scope_world_id, scope_channel_id, scope_instance_id) =
        scope_storage_v2(record);
    let encoded_record = encode_record_v2(record).to_string();
    let attempt_ref = identity.reconnect_attempt_ref().to_be_bytes();
    let transport_ref = record.connection().transport_ref().to_bytes();
    let inserted = sqlx::query(
        "INSERT INTO game_durability_reconnect_attempts (\
            game_session_id, reconnect_attempt_ref, control_loss_epoch, transport_ref, \
            account_id, character_id, world_id, runtime_scope_kind, runtime_scope_world_id, \
            runtime_scope_channel_id, runtime_scope_instance_id, fnd02_next_command_id, \
            record_json, state\
         ) VALUES (\
            encode($1, 'hex')::uuid, $2, $3::text::numeric(20, 0), $4, $5::text::uuid, \
            encode($6, 'hex')::uuid, encode($7, 'hex')::uuid, $8, \
            encode($9, 'hex')::uuid, encode($10, 'hex')::uuid, \
            encode($11, 'hex')::uuid, $12::text::numeric(20, 0), $13, $14\
         )",
    )
    .bind(identity.game_session_id().as_bytes().as_slice())
    .bind(attempt_ref.as_slice())
    .bind(record.continuity().control_loss_epoch().get().to_string())
    .bind(transport_ref.as_slice())
    .bind(identity.account_id())
    .bind(identity.character_id().as_bytes().as_slice())
    .bind(identity.world_id().as_bytes().as_slice())
    .bind(scope_kind)
    .bind(scope_world_id.as_slice())
    .bind(scope_channel_id.as_deref())
    .bind(scope_instance_id.as_deref())
    .bind(record.fnd02().next_command_id().get().to_string())
    .bind(encoded_record)
    .bind(state)
    .execute(&mut **transaction)
    .await?;
    if inserted.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }

    for pending in record.fnd02().pending() {
        let disposition = match pending.disposition() {
            PendingCommandDispositionV1::PendingOriginal => V2_PENDING_ORIGINAL,
            PendingCommandDispositionV1::TerminalOutcomeRetained => V2_TERMINAL_OUTCOME_RETAINED,
        };
        sqlx::query(
            "INSERT INTO game_durability_reconnect_pending_commands (\
                game_session_id, reconnect_attempt_ref, command_id, disposition\
             ) VALUES (encode($1, 'hex')::uuid, $2, $3::text::numeric(20, 0), $4)",
        )
        .bind(identity.game_session_id().as_bytes().as_slice())
        .bind(attempt_ref.as_slice())
        .bind(pending.command_id().get().to_string())
        .bind(disposition)
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}

async fn database_now_v2(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<i64, DurabilityError> {
    sqlx::query_scalar("SELECT FLOOR(EXTRACT(EPOCH FROM clock_timestamp()))::BIGINT")
        .fetch_one(&mut **transaction)
        .await
        .map_err(DurabilityError::from)
}

fn terminal_disposition_from_state(
    state: i16,
) -> Result<ReconnectDurableTerminalDispositionV1, DurabilityError> {
    match state {
        V2_COLLISION_TERMINAL => Ok(ReconnectDurableTerminalDispositionV1::TransportRefCollision),
        V2_CONCURRENT_TERMINAL => Ok(ReconnectDurableTerminalDispositionV1::ConcurrentPrepared),
        V2_STALE_TERMINAL => Ok(ReconnectDurableTerminalDispositionV1::StaleAuthority),
        _ => Err(DurabilityError::InvalidStoredState),
    }
}

fn scope_storage_v2(record: &ReconnectDurabilityRecordV1) -> V2ScopeStorage {
    match record.identity().runtime_scope() {
        RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } => (
            V2_CHANNEL_SCOPE,
            world_id.as_bytes().to_vec(),
            Some(channel_id.as_bytes().to_vec()),
            None,
        ),
        RuntimeScopeRefV1::Instance {
            world_id,
            instance_id,
        } => (
            V2_INSTANCE_SCOPE,
            world_id.as_bytes().to_vec(),
            None,
            Some(instance_id.to_vec()),
        ),
    }
}

fn encode_record_v2(record: &ReconnectDurabilityRecordV1) -> serde_json::Value {
    let identity = record.identity();
    let connection = record.connection();
    let authority = record.authority();
    let continuity = record.continuity();
    let scope = match identity.runtime_scope() {
        RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } => json!({
            "kind": "channel",
            "world_id": world_id.as_bytes(),
            "channel_id": channel_id.as_bytes(),
        }),
        RuntimeScopeRefV1::Instance {
            world_id,
            instance_id,
        } => json!({
            "kind": "instance",
            "world_id": world_id.as_bytes(),
            "instance_id": instance_id,
        }),
    };
    let protection = match continuity.protection_entitlement() {
        ProtectionEntitlementV1::Unused => json!({ "state": "unused" }),
        ProtectionEntitlementV1::Fenced { generation } => {
            json!({ "state": "fenced", "generation": generation })
        }
    };
    let proof = match record.proof() {
        ReconnectProofV1::FastReconnect {
            reconnect_proof_generation,
        } => json!({ "class": "fast_reconnect", "generation": reconnect_proof_generation }),
        ReconnectProofV1::ReauthenticatedRecovery {
            recovery_grant_nonce,
        } => json!({
            "class": "reauthenticated_recovery",
            "recovery_grant_nonce": recovery_grant_nonce,
        }),
    };
    let fnd02 = record.fnd02();
    let compatibility = record.compatibility();
    json!({
        "version": record.version(),
        "identity": {
            "game_session_id": identity.game_session_id().as_bytes(),
            "reconnect_attempt_ref": identity.reconnect_attempt_ref().to_be_bytes(),
            "account_id": identity.account_id(),
            "character_id": identity.character_id().as_bytes(),
            "world_id": identity.world_id().as_bytes(),
            "runtime_scope": scope,
        },
        "connection": {
            "predecessor_generation": connection.predecessor().get(),
            "candidate_generation": connection.candidate().get(),
            "transport_ref": connection.transport_ref().to_bytes(),
        },
        "authority": {
            "character_lease_generation": authority.character_lease_generation(),
            "scope_ownership_generation": authority.scope_ownership_generation().get(),
            "expected_session_state": "reconnectable",
            "expected_no_current_controller": true,
        },
        "continuity": {
            "control_loss_epoch": continuity.control_loss_epoch().get(),
            "original_grace_deadline": continuity.original_grace_deadline(),
            "prepared_deadline": continuity.prepared_deadline(),
            "protection_entitlement": protection,
        },
        "proof": proof,
        "fnd02": {
            "next_command_id": fnd02.next_command_id().get(),
            "pending": fnd02.pending().iter().map(|pending| json!({
                "command_id": pending.command_id().get(),
                "disposition": match pending.disposition() {
                    PendingCommandDispositionV1::PendingOriginal => "pending_original",
                    PendingCommandDispositionV1::TerminalOutcomeRetained => "terminal_outcome_retained",
                },
            })).collect::<Vec<_>>(),
            "server_sequence": fnd02.server_sequence(),
            "domain_revisions": fnd02.domain_revisions().iter().map(|revision| json!({
                "domain_id": revision.domain_id(),
                "revision": revision.revision(),
            })).collect::<Vec<_>>(),
        },
        "compatibility": {
            "protocol_major": compatibility.protocol_major(),
            "transport_profile": compatibility.transport_profile(),
            "ruleset_revision": compatibility.ruleset_revision(),
            "content_revision": compatibility.content_revision(),
            "map_revision": compatibility.map_revision(),
            "world_policy_revision": compatibility.world_policy_revision(),
            "account_security_generation": compatibility.account_security_generation(),
            "platform_security_evidence": encode_evidence_v2(compatibility.platform_security_evidence()),
            "proof_trust_evidence": encode_evidence_v2(compatibility.proof_trust_evidence()),
            "credential_expiration": compatibility.credential_expiration(),
        },
    })
}

fn encode_evidence_v2(
    evidence: &oteryn_game_server::foundation::AuthorityEvidenceFenceV1,
) -> serde_json::Value {
    json!({
        "authority": evidence.authority(),
        "purpose": evidence.purpose(),
        "scope": evidence.scope(),
        "source_revision": evidence.source_revision(),
        "decision_identity": evidence.decision_identity(),
        "source_observed_at": evidence.source_observed_at(),
    })
}

#[cfg(test)]
mod terminal_replacement_schema_red_tests {
    const MIGRATION: &str = include_str!("../../migrations/0001_admission_reconnect_journal.sql");

    #[test]
    fn terminal_replacement_forward_syncs_lagging_scope_fence_atomically() {
        assert!(
            MIGRATION.contains(
                "session_state SMALLINT NOT NULL DEFAULT 1 CHECK (session_state BETWEEN 1 AND 3)"
            ),
            "terminal predecessor replacement requires an explicit durable TERMINAL session state before any forward scope synchronization can be committed"
        );
        assert!(
            MIGRATION.contains("predecessor_scope_ownership_generation NUMERIC(20, 0) NOT NULL"),
            "replacement receipt must retain the exact Foundation-authorized predecessor scope fence"
        );
    }

    #[test]
    fn terminal_replacement_rejects_scope_fence_ahead_of_foundation_authority() {
        assert!(
            MIGRATION.contains("game_durability_session_replacements"),
            "scope comparison must be bound to a durable predecessor-to-candidate replacement receipt"
        );
    }

    #[test]
    fn terminal_replacement_rejects_live_or_mismatched_predecessor_without_mutation() {
        assert!(
            MIGRATION.contains("game_durability_one_nonterminal_session_per_character"),
            "one-live-session-per-character must remain database-enforced while terminal history is retained"
        );
        assert!(
            !MIGRATION.contains("    UNIQUE (character_id),"),
            "unconditional actor uniqueness permanently binds a character to its first historical GameSession"
        );
    }

    #[test]
    fn terminal_replacement_lost_response_replays_only_exact_receipt_binding() {
        for required in [
            "character_id UUID NOT NULL",
            "predecessor_game_session_id UUID NOT NULL",
            "candidate_game_session_id UUID NOT NULL",
            "PRIMARY KEY (character_id, predecessor_game_session_id, candidate_game_session_id)",
        ] {
            assert!(
                MIGRATION.contains(required),
                "lost-response replacement replay requires exact receipt binding field: {required}"
            );
        }
    }

    #[test]
    fn terminal_replacement_conflicting_receipt_binding_fails_closed() {
        assert!(
            MIGRATION.contains("UNIQUE (character_id, candidate_game_session_id)"),
            "a candidate cannot be replay-equivalent to multiple predecessor bindings"
        );
    }

    #[test]
    fn terminal_replacement_fences_predecessor_prepared_attempt_against_late_commit() {
        assert!(
            MIGRATION.contains(
                "session_state SMALLINT NOT NULL DEFAULT 1 CHECK (session_state BETWEEN 1 AND 3)"
            ),
            "the predecessor needs a durable terminal state that late COMMIT validation can fail closed against"
        );
    }

    #[test]
    fn terminal_replacement_mid_transaction_failure_rolls_back_predecessor_and_candidate() {
        assert!(
            MIGRATION.contains("game_durability_session_replacements"),
            "replacement transaction rollback cannot be proven until receipt persistence exists in the same ledger"
        );
    }

    #[test]
    fn collision_existing_terminal_replay_preserves_typed_collision_reason() {
        assert!(
            MIGRATION.contains("game_durability_reconnect_attempts"),
            "typed terminal replay must continue to derive from the durable attempt ledger"
        );
        assert!(
            MIGRATION.contains("state SMALLINT NOT NULL CHECK (state BETWEEN 1 AND 5)"),
            "existing terminal attempt classes must remain durably distinguishable"
        );
    }

    #[test]
    fn v2_reconciliation_round_trips_collision_concurrent_and_stale_distinctly() {
        assert!(
            MIGRATION.contains("state SMALLINT NOT NULL CHECK (state BETWEEN 1 AND 5)"),
            "V2 reconciliation requires the existing distinct durable terminal attempt states"
        );
    }

    #[test]
    fn concurrent_terminal_replacement_has_exactly_one_candidate_winner() {
        assert!(
            MIGRATION.contains(
                "CREATE UNIQUE INDEX game_durability_one_nonterminal_session_per_character"
            ),
            "concurrent terminal replacement requires a database-enforced unique nonterminal actor anchor"
        );
        assert!(
            MIGRATION.contains("WHERE session_state IN (1, 2)"),
            "the unique actor anchor must exclude terminal historical rows"
        );
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::too_many_arguments)]
mod terminal_replacement_foundation_red_tests {
    use oteryn_game_server::foundation::{
        AccountPresenceClaimV1, AuthenticatedTransportRefV1, AuthorityEvidenceFenceV1, ChannelId,
        CharacterId, CharacterLease, CharacterWorldEligibilityClaimV1, CommandId,
        ConnectionGeneration, ControlLossEpochRefV1, Fnd02ReconciliationFenceV1,
        FreshAdmissionCommit, FreshAdmissionFacts, GameSessionAuthoritySnapshot, GameSessionId,
        GameSessionState, PendingCommandDispositionV1, PendingCommandReconciliationV1,
        ProtectionEntitlementV1, ReconnectAttemptBudgetV1, ReconnectAttemptRef,
        ReconnectAttemptReservationV1, ReconnectAuthorityFenceV1, ReconnectCandidateBindingV1,
        ReconnectCompatibilityEvidenceV1, ReconnectConnectionFenceV1, ReconnectContinuityV1,
        ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1, ReconnectDurabilityFlowV1,
        ReconnectDurabilityFlowV2, ReconnectDurabilityPhaseV1, ReconnectDurabilityRecordV1,
        ReconnectDurableOutcomeV2, ReconnectDurableReconciliationSnapshotV1,
        ReconnectDurableReconciliationSnapshotV2, ReconnectDurableTerminalDispositionV1,
        ReconnectIdentityV1, ReconnectPrepareActionV1, ReconnectPrepareCompletionV1,
        ReconnectPrepareCompletionV2, ReconnectPrepareDispositionV1, ReconnectPrepareDispositionV2,
        ReconnectProjectionDecisionV1, ReconnectProjectionDecisionV2, ReconnectProofV1,
        RuntimeScopeRefV1, ScopeOwnershipGeneration, StateDomainRevisionV1,
        TerminalGameSessionReplacementAuthorizationV1, WorldId,
    };

    const ACCOUNT: &str = "123e4567-e89b-12d3-a456-426614174000";
    const OTHER_ACCOUNT: &str = "123e4567-e89b-12d3-a456-426614174001";

    fn exact_current_authority(
        record: &ReconnectDurabilityRecordV1,
        observed_at: i64,
    ) -> Result<ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1> {
        current_authority(
            record,
            Some(AccountPresenceClaimV1::new(
                record.identity().account_id(),
                record.identity().character_id(),
            )?),
            observed_at,
        )
    }

    fn current_authority(
        record: &ReconnectDurabilityRecordV1,
        current_account_presence: Option<AccountPresenceClaimV1>,
        observed_at: i64,
    ) -> Result<ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1> {
        ReconnectCurrentAuthorityV1::from_current_facts(
            record,
            current_account_presence,
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

    #[test]
    fn v1_committed_reconciliation_requires_complete_current_authority() {
        let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
        let committed = ReconnectDurableReconciliationSnapshotV1::committed(record.clone());

        let (mut exact_flow, exact_request) = ReconnectDurabilityFlowV1::begin(record.clone());
        exact_flow
            .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                &exact_request,
                ReconnectPrepareDispositionV1::Ambiguous,
            ))
            .expect("ambiguous prepare enters reconciliation");
        let exact = exact_current_authority(&record, 105).expect("exact current authority");
        assert_eq!(
            exact_flow.accept_reconciliation(committed.clone(), exact),
            Ok(ReconnectProjectionDecisionV1::InstallController {
                generation: record.connection().candidate(),
                transport_ref: record.connection().transport_ref(),
            })
        );

        let (mut stale_flow, stale_request) = ReconnectDurabilityFlowV1::begin(record.clone());
        stale_flow
            .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                &stale_request,
                ReconnectPrepareDispositionV1::Ambiguous,
            ))
            .expect("ambiguous prepare enters reconciliation");
        let stale = current_authority(&record, None, 104).expect("stale current authority");
        assert_eq!(
            stale_flow.accept_reconciliation(committed, stale),
            Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
        );
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

    fn candidate_record(
        session_raw: u64,
        account_id: &str,
        character_raw: u64,
        world_raw: u64,
        predecessor_generation: u64,
        lease_generation: u64,
        scope_generation: u64,
        attempt: u64,
    ) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        candidate_record_with_evidence(
            session_raw,
            account_id,
            character_raw,
            world_raw,
            predecessor_generation,
            lease_generation,
            scope_generation,
            attempt,
            100,
            101,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn candidate_record_with_evidence(
        session_raw: u64,
        account_id: &str,
        character_raw: u64,
        world_raw: u64,
        predecessor_generation: u64,
        lease_generation: u64,
        scope_generation: u64,
        attempt: u64,
        platform_source_observed_at: i64,
        trust_source_observed_at: i64,
    ) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        let world_id = world(world_raw)?;
        let identity = ReconnectIdentityV1::new(
            game_session(session_raw)?,
            ReconnectAttemptRef::new(attempt)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            account_id,
            character(character_raw)?,
            world_id,
            RuntimeScopeRefV1::channel(world_id, channel(13)?),
        )?;
        let connection = ReconnectConnectionFenceV1::new(
            ConnectionGeneration::new(predecessor_generation)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            ConnectionGeneration::new(predecessor_generation + 1)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            AuthenticatedTransportRefV1::decode(&[0x71; 16])
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?;
        let authority = ReconnectAuthorityFenceV1::new(
            lease_generation,
            ScopeOwnershipGeneration::new(scope_generation)
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
            platform_source_observed_at,
        )?;
        let trust = AuthorityEvidenceFenceV1::new(
            "proof-trust",
            "reconnect",
            "recovery-key",
            "trust:21",
            "decision:trust:21",
            trust_source_observed_at,
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

    fn prepared_v1_flow(record: &ReconnectDurabilityRecordV1) -> ReconnectDurabilityFlowV1 {
        let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
        flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
            &request,
            ReconnectPrepareDispositionV1::Prepared,
        ))
        .expect("prepare completion");
        flow
    }

    fn prepared_v2_flow(record: &ReconnectDurabilityRecordV1) -> ReconnectDurabilityFlowV2 {
        let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
        budget
            .reserve(
                record.identity().reconnect_attempt_ref(),
                record.connection().transport_ref(),
            )
            .expect("reserve");
        let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
        flow.accept_prepare_completion(
            ReconnectPrepareCompletionV2::for_request(
                &request,
                ReconnectPrepareDispositionV2::Prepared,
            ),
            &mut budget,
        )
        .expect("prepare completion");
        flow
    }

    #[test]
    fn commit_authorization_rejects_future_authenticated_evidence_and_accepts_equal_timestamps() {
        let exact = candidate_record_with_evidence(20, ACCOUNT, 11, 12, 7, 9, 10, 1, 105, 105)
            .expect("exact record");
        assert!(
            prepared_v1_flow(&exact)
                .authorize_commit(exact_current_authority(&exact, 105).expect("current"), 105)
                .is_ok()
        );
        assert!(
            prepared_v2_flow(&exact)
                .authorize_commit(exact_current_authority(&exact, 105).expect("current"), 105)
                .is_ok()
        );

        for (platform_at, trust_at) in [(106, 105), (105, 106)] {
            let record = candidate_record_with_evidence(
                20,
                ACCOUNT,
                11,
                12,
                7,
                9,
                10,
                1,
                platform_at,
                trust_at,
            )
            .expect("future-evidence record");
            assert_eq!(
                prepared_v1_flow(&record).authorize_commit(
                    exact_current_authority(&record, 105).expect("current"),
                    105,
                ),
                Err(ReconnectDurabilityErrorV1::StaleAuthority)
            );
            assert_eq!(
                prepared_v2_flow(&record).authorize_commit(
                    exact_current_authority(&record, 105).expect("current"),
                    105,
                ),
                Err(ReconnectDurabilityErrorV1::StaleAuthority)
            );
        }
    }

    #[test]
    fn committed_reconciliation_rejects_authority_observed_before_authenticated_evidence() {
        for (platform_at, trust_at) in [(106, 105), (105, 106)] {
            let record = candidate_record_with_evidence(
                20,
                ACCOUNT,
                11,
                12,
                7,
                9,
                10,
                1,
                platform_at,
                trust_at,
            )
            .expect("future-evidence record");
            let committed = ReconnectDurableReconciliationSnapshotV1::committed(record.clone());
            let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
            flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                &request,
                ReconnectPrepareDispositionV1::Ambiguous,
            ))
            .expect("ambiguous prepare");
            assert_eq!(
                flow.accept_reconciliation(
                    committed,
                    exact_current_authority(&record, 105).expect("current"),
                ),
                Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
            );
        }
    }

    fn predecessor_snapshot(
        state: GameSessionState,
        current_transport: Option<AuthenticatedTransportRefV1>,
        current_scope: u64,
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
            state,
            ConnectionGeneration::new(7).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            current_transport,
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
        .with_control_loss_continuity(ControlLossEpochRefV1::new(3)?, 120)
    }

    fn authorize(
        snapshot: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        candidate: &ReconnectDurabilityRecordV1,
        expected_predecessor: GameSessionId,
        expected_candidate: GameSessionId,
    ) -> Result<TerminalGameSessionReplacementAuthorizationV1, ReconnectDurabilityErrorV1> {
        TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
            ACCOUNT,
            Some(&AccountPresenceClaimV1::new(
                candidate.identity().account_id(),
                candidate.identity().character_id(),
            )?),
            expected_predecessor,
            expected_candidate,
            snapshot,
            candidate,
        )
    }

    #[test]
    fn v2_replacement_root_ready_miss_is_fail_closed_without_connect() {
        use std::net::{IpAddr, Ipv4Addr};

        let candidate =
            candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("candidate record");
        let authorization = authorize(
            predecessor_snapshot(GameSessionState::Terminal, None, 10).expect("snapshot"),
            &candidate,
            game_session(10).expect("predecessor session"),
            game_session(20).expect("candidate session"),
        )
        .expect("replacement authorization");
        let request = ReconnectDurabilityFlowV2::begin(candidate, Some(authorization)).1;

        let root = super::DurabilityRoot::new(
            super::DurabilityRootConfig::new_with_isolated_test_ledger(
                IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)),
                5432,
                "db.example",
                "oteryn",
                "explicit",
                "test-secret",
                b"-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n",
            )
            .expect("root config"),
        )
        .expect("durability root");
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(async {
                let journal = super::AdmissionReconnectJournalV2::from_root(root.clone());

                assert!(matches!(
                    journal.prepare(&request).await,
                    Err(super::DurabilityError::RootUnavailable)
                ));
                assert!(!root.is_ready());
                assert!(root.has_ready_demand());
                tokio::task::yield_now().await;
                assert!(
                    !root.is_ready(),
                    "V2 replacement ready miss must not establish a connection in the background"
                );
            });
        drop(root);
    }

    #[test]
    fn v2_final_revalidation_accepts_external_current_facts_and_rejects_changed_authority() {
        let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
        let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
        budget
            .reserve(
                record.identity().reconnect_attempt_ref(),
                record.connection().transport_ref(),
            )
            .expect("reserve");
        let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
        flow.accept_prepare_completion(
            ReconnectPrepareCompletionV2::for_request(
                &request,
                ReconnectPrepareDispositionV2::Prepared,
            ),
            &mut budget,
        )
        .expect("prepare completion");

        let exact_current = ReconnectCurrentAuthorityV1::from_current_facts(
            &record,
            Some(
                AccountPresenceClaimV1::new(
                    record.identity().account_id(),
                    record.identity().character_id(),
                )
                .expect("presence"),
            ),
            Some(CharacterWorldEligibilityClaimV1::new(
                record.identity().character_id(),
                record.identity().world_id(),
            )),
            Some(
                ReconnectCandidateBindingV1::new(
                    record.identity().game_session_id(),
                    record.identity().reconnect_attempt_ref(),
                    record.connection().candidate(),
                    record.connection().transport_ref(),
                    record.continuity().prepared_deadline(),
                )
                .expect("candidate binding"),
            ),
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
            105,
        )
        .expect("exact current authority");
        assert!(flow.authorize_commit(exact_current, 104).is_ok());

        let mut changed_budget =
            ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
        changed_budget
            .reserve(
                record.identity().reconnect_attempt_ref(),
                record.connection().transport_ref(),
            )
            .expect("changed reserve");
        let (mut changed_flow, changed_request) =
            ReconnectDurabilityFlowV2::begin(record.clone(), None);
        changed_flow
            .accept_prepare_completion(
                ReconnectPrepareCompletionV2::for_request(
                    &changed_request,
                    ReconnectPrepareDispositionV2::Prepared,
                ),
                &mut changed_budget,
            )
            .expect("changed prepare completion");
        let changed_authority = ReconnectAuthorityFenceV1::new(
            record.authority().character_lease_generation() + 1,
            record.authority().scope_ownership_generation(),
        )
        .expect("changed authority");
        let changed_current = ReconnectCurrentAuthorityV1::from_current_facts(
            &record,
            Some(
                AccountPresenceClaimV1::new(
                    record.identity().account_id(),
                    record.identity().character_id(),
                )
                .expect("presence"),
            ),
            Some(CharacterWorldEligibilityClaimV1::new(
                record.identity().character_id(),
                record.identity().world_id(),
            )),
            Some(
                ReconnectCandidateBindingV1::new(
                    record.identity().game_session_id(),
                    record.identity().reconnect_attempt_ref(),
                    record.connection().candidate(),
                    record.connection().transport_ref(),
                    record.continuity().prepared_deadline(),
                )
                .expect("candidate binding"),
            ),
            record.identity().runtime_scope(),
            record.connection().predecessor(),
            changed_authority,
            record.continuity().control_loss_epoch(),
            record.continuity().original_grace_deadline(),
            record.proof().clone(),
            record.fnd02().clone(),
            record.compatibility().clone(),
            GameSessionState::Reconnectable,
            false,
            105,
        )
        .expect("changed current authority");

        assert_eq!(
            changed_flow.authorize_commit(changed_current, 104),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );
    }

    #[test]
    fn v2_final_authority_revalidation_requires_current_account_presence() {
        let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
        let exact_presence = AccountPresenceClaimV1::new(
            record.identity().account_id(),
            record.identity().character_id(),
        )
        .expect("account presence");
        let reassigned_presence =
            AccountPresenceClaimV1::new(ACCOUNT, character(99).expect("character"))
                .expect("reassigned account presence");

        let current = |presence| {
            ReconnectCurrentAuthorityV1::from_current_facts(
                &record,
                presence,
                Some(CharacterWorldEligibilityClaimV1::new(
                    record.identity().character_id(),
                    record.identity().world_id(),
                )),
                Some(
                    ReconnectCandidateBindingV1::new(
                        record.identity().game_session_id(),
                        record.identity().reconnect_attempt_ref(),
                        record.connection().candidate(),
                        record.connection().transport_ref(),
                        record.continuity().prepared_deadline(),
                    )
                    .expect("candidate binding"),
                ),
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
                105,
            )
            .expect("current authority")
        };
        let flow = || {
            let mut budget =
                ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
            budget
                .reserve(
                    record.identity().reconnect_attempt_ref(),
                    record.connection().transport_ref(),
                )
                .expect("reserve");
            let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
            flow.accept_prepare_completion(
                ReconnectPrepareCompletionV2::for_request(
                    &request,
                    ReconnectPrepareDispositionV2::Prepared,
                ),
                &mut budget,
            )
            .expect("prepare completion");
            flow
        };

        assert!(
            flow()
                .authorize_commit(current(Some(exact_presence.clone())), 104)
                .is_ok()
        );
        assert_eq!(
            flow().authorize_commit(current(None), 104),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );
        assert_eq!(
            flow().authorize_commit(current(Some(reassigned_presence.clone())), 104),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );

        let committed = ReconnectDurableReconciliationSnapshotV2::new(
            record.clone(),
            ReconnectDurableOutcomeV2::Committed {
                current_generation: record.connection().candidate(),
                current_transport_ref: record.connection().transport_ref(),
            },
        );
        let reconcile = |presence| {
            let mut budget =
                ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
            budget
                .reserve(
                    record.identity().reconnect_attempt_ref(),
                    record.connection().transport_ref(),
                )
                .expect("reserve");
            let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
            flow.accept_prepare_completion(
                ReconnectPrepareCompletionV2::for_request(
                    &request,
                    ReconnectPrepareDispositionV2::Ambiguous,
                ),
                &mut budget,
            )
            .expect("prepare completion");
            flow.accept_reconciliation(committed.clone(), current(presence), &mut budget)
        };
        assert!(matches!(
            reconcile(Some(exact_presence)),
            Ok(ReconnectProjectionDecisionV2::InstallController { .. })
        ));
        assert_eq!(
            reconcile(None),
            Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
        );
        assert_eq!(
            reconcile(Some(reassigned_presence)),
            Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
        );
    }

    #[test]
    fn terminal_replacement_authorization_rejects_candidate_loss_epoch_mismatch() {
        let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
        let mismatched_continuity = ReconnectContinuityV1::new(
            ControlLossEpochRefV1::new(4).expect("epoch"),
            record.continuity().original_grace_deadline(),
            record.continuity().prepared_deadline(),
            record.continuity().protection_entitlement(),
        )
        .expect("continuity");
        let mismatched = ReconnectDurabilityRecordV1::new(
            record.identity().clone(),
            record.connection(),
            record.authority(),
            mismatched_continuity,
            record.proof().clone(),
            record.fnd02().clone(),
            record.compatibility().clone(),
        )
        .expect("mismatched record");

        assert!(
            authorize(
                predecessor_snapshot(GameSessionState::Terminal, None, 10).expect("snapshot"),
                &mismatched,
                game_session(10).expect("predecessor"),
                game_session(20).expect("candidate"),
            )
            .is_err()
        );
    }

    #[test]
    fn terminal_replacement_authorization_rejects_candidate_original_deadline_mismatch() {
        let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
        let mismatched_continuity = ReconnectContinuityV1::new(
            record.continuity().control_loss_epoch(),
            record.continuity().original_grace_deadline() + 1,
            record.continuity().prepared_deadline(),
            record.continuity().protection_entitlement(),
        )
        .expect("continuity");
        let mismatched = ReconnectDurabilityRecordV1::new(
            record.identity().clone(),
            record.connection(),
            record.authority(),
            mismatched_continuity,
            record.proof().clone(),
            record.fnd02().clone(),
            record.compatibility().clone(),
        )
        .expect("mismatched record");

        assert!(
            authorize(
                predecessor_snapshot(GameSessionState::Terminal, None, 10).expect("snapshot"),
                &mismatched,
                game_session(10).expect("predecessor"),
                game_session(20).expect("candidate"),
            )
            .is_err()
        );
    }

    #[test]
    fn terminal_replacement_authorization_requires_terminal_transportless_predecessor() {
        let candidate = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 11, 1).expect("candidate");
        let predecessor = game_session(10).expect("predecessor");
        let candidate_id = game_session(20).expect("candidate id");
        assert!(
            authorize(
                predecessor_snapshot(
                    GameSessionState::Active,
                    Some(AuthenticatedTransportRefV1::decode(&[0x70; 16]).expect("transport"),),
                    11,
                )
                .expect("snapshot"),
                &candidate,
                predecessor,
                candidate_id,
            )
            .is_err()
        );
        assert!(
            authorize(
                predecessor_snapshot(GameSessionState::Reconnectable, None, 11).expect("snapshot"),
                &candidate,
                predecessor,
                candidate_id,
            )
            .is_err()
        );
        assert!(
            authorize(
                predecessor_snapshot(
                    GameSessionState::Terminal,
                    Some(AuthenticatedTransportRefV1::decode(&[0x70; 16]).expect("transport"),),
                    11,
                )
                .expect("snapshot"),
                &candidate,
                predecessor,
                candidate_id,
            )
            .is_err()
        );
        assert!(
            authorize(
                predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
                &candidate,
                predecessor,
                candidate_id,
            )
            .is_ok()
        );
    }

    #[test]
    fn terminal_replacement_authorization_carries_current_scope_not_only_committed_scope() {
        let candidate = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 11, 1).expect("candidate");
        let authorization = authorize(
            predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
            &candidate,
            game_session(10).expect("predecessor"),
            game_session(20).expect("candidate"),
        )
        .expect("terminal authorization");
        assert_eq!(
            authorization
                .predecessor_current_scope_ownership_generation()
                .get(),
            11
        );
    }

    #[test]
    fn terminal_replacement_authorization_rejects_predecessor_session_mismatch() {
        let candidate = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 11, 1).expect("candidate");
        assert!(
            authorize(
                predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
                &candidate,
                game_session(99).expect("wrong predecessor"),
                game_session(20).expect("candidate"),
            )
            .is_err()
        );
    }

    #[test]
    fn terminal_replacement_authorization_rejects_predecessor_connection_generation_mismatch() {
        let candidate = candidate_record(20, ACCOUNT, 11, 12, 6, 9, 11, 1).expect("candidate");
        assert!(
            authorize(
                predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
                &candidate,
                game_session(10).expect("predecessor"),
                game_session(20).expect("candidate"),
            )
            .is_err()
        );
    }

    #[test]
    fn terminal_replacement_authorization_rejects_predecessor_lease_generation_mismatch() {
        let candidate = candidate_record(20, ACCOUNT, 11, 12, 7, 10, 11, 1).expect("candidate");
        assert!(
            authorize(
                predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
                &candidate,
                game_session(10).expect("predecessor"),
                game_session(20).expect("candidate"),
            )
            .is_err()
        );
    }

    #[test]
    fn terminal_replacement_authorization_rejects_candidate_session_mismatch() {
        let candidate = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 11, 1).expect("candidate");
        assert!(
            authorize(
                predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
                &candidate,
                game_session(10).expect("predecessor"),
                game_session(21).expect("wrong candidate"),
            )
            .is_err()
        );
    }

    #[test]
    fn terminal_replacement_authorization_rejects_candidate_account_mismatch() {
        let candidate =
            candidate_record(20, OTHER_ACCOUNT, 11, 12, 7, 9, 11, 1).expect("candidate");
        assert!(
            authorize(
                predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
                &candidate,
                game_session(10).expect("predecessor"),
                game_session(20).expect("candidate"),
            )
            .is_err()
        );
    }

    #[test]
    fn terminal_replacement_authorization_rejects_candidate_character_mismatch() {
        let candidate = candidate_record(20, ACCOUNT, 99, 12, 7, 9, 11, 1).expect("candidate");
        assert!(
            authorize(
                predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
                &candidate,
                game_session(10).expect("predecessor"),
                game_session(20).expect("candidate"),
            )
            .is_err()
        );
    }

    #[test]
    fn terminal_replacement_authorization_rejects_candidate_world_mismatch() {
        let candidate = candidate_record(20, ACCOUNT, 11, 99, 7, 9, 11, 1).expect("candidate");
        assert!(
            authorize(
                predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"),
                &candidate,
                game_session(10).expect("predecessor"),
                game_session(20).expect("candidate"),
            )
            .is_err()
        );
    }

    #[test]
    fn generic_v1_existing_terminal_requires_typed_same_attempt_reconciliation() {
        let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
        let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record);
        assert_eq!(
            flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                &request,
                ReconnectPrepareDispositionV1::ExistingTerminal,
            ))
            .expect("completion"),
            ReconnectPrepareActionV1::ReconcileSameAttempt
        );
        assert_eq!(
            flow.phase(),
            ReconnectDurabilityPhaseV1::ReconciliationRequired
        );
    }

    #[test]
    fn v2_direct_existing_terminal_collision_marks_budget_and_respects_capacity() {
        let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
        let attempt = record.identity().reconnect_attempt_ref();
        let transport = record.connection().transport_ref();
        let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
        assert_eq!(
            budget.reserve(attempt, transport).expect("reserve"),
            ReconnectAttemptReservationV1::New
        );
        let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record, None);
        flow.accept_prepare_completion(
            ReconnectPrepareCompletionV2::for_request(
                &request,
                ReconnectPrepareDispositionV2::ExistingTerminal {
                    disposition: ReconnectDurableTerminalDispositionV1::TransportRefCollision,
                },
            ),
            &mut budget,
        )
        .expect("typed replay");
        assert!(budget.replacement_allowed_after_collision(attempt));
    }

    #[test]
    fn v2_direct_existing_terminal_noncollision_never_unlocks_fresh_attempt() {
        for disposition in [
            ReconnectDurableTerminalDispositionV1::ConcurrentPrepared,
            ReconnectDurableTerminalDispositionV1::StaleAuthority,
        ] {
            let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
            let attempt = record.identity().reconnect_attempt_ref();
            let transport = record.connection().transport_ref();
            let mut budget =
                ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
            budget.reserve(attempt, transport).expect("reserve");
            let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record, None);
            flow.accept_prepare_completion(
                ReconnectPrepareCompletionV2::for_request(
                    &request,
                    ReconnectPrepareDispositionV2::ExistingTerminal { disposition },
                ),
                &mut budget,
            )
            .expect("typed replay");
            assert!(!budget.replacement_allowed_after_collision(attempt));
        }
    }

    #[test]
    fn v2_reconciliation_preserves_all_terminal_dispositions_and_collision_only_remint() {
        for (disposition, allows_replacement) in [
            (
                ReconnectDurableTerminalDispositionV1::TransportRefCollision,
                true,
            ),
            (
                ReconnectDurableTerminalDispositionV1::ConcurrentPrepared,
                false,
            ),
            (ReconnectDurableTerminalDispositionV1::StaleAuthority, false),
        ] {
            let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
            let attempt = record.identity().reconnect_attempt_ref();
            let transport = record.connection().transport_ref();
            let mut budget =
                ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
            budget.reserve(attempt, transport).expect("reserve");
            let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
            flow.accept_prepare_completion(
                ReconnectPrepareCompletionV2::for_request(
                    &request,
                    ReconnectPrepareDispositionV2::Ambiguous,
                ),
                &mut budget,
            )
            .expect("ambiguous");
            let snapshot = ReconnectDurableReconciliationSnapshotV2::new(
                record.clone(),
                ReconnectDurableOutcomeV2::Terminal { disposition },
            );
            let decision = flow
                .accept_reconciliation(
                    snapshot,
                    exact_current_authority(&record, 105).expect("current authority"),
                    &mut budget,
                )
                .expect("typed reconciliation");
            assert_eq!(decision.terminal_disposition(), Some(disposition));
            assert_eq!(
                budget.replacement_allowed_after_collision(attempt),
                allows_replacement
            );
        }
    }
}
