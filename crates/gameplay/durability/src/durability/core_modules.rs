// PostgreSQL-backed, journal-only reconnect durability boundary.
//
// This module deliberately owns persistence/classification only. Foundation
// constructs and revalidates reconnect authority; the runtime must submit the
// resulting request asynchronously and consume its completion as new input.

// Keep sorted by module name; insert in order, never append.
pub mod account_achievement;
pub mod account_characters_projection;
pub mod admission_authority_guards;
pub mod admission_journal;
pub mod bank;
pub mod bank_audit;
mod bank_credit_headroom;
pub mod bestiary_progress;
pub mod blessing_purchase;
#[allow(
    dead_code,
    reason = "APP-1 production caller awaits admitted content, sex and grant owners"
)]
pub mod character_appearance;
pub mod character_authority;
pub mod character_build;
pub mod character_chat_name;
pub mod character_chat_recipient;
pub mod character_command;
pub mod character_cyclopedia;
pub mod character_death;
pub mod character_depot;
pub mod character_equipment;
mod character_experience_transaction;
pub mod character_familiar;
pub mod character_forge_dust;
pub mod character_forge_dust_audit;
pub mod character_inbox;
pub mod character_last_position;
pub mod character_proficiency;
pub mod character_proficiency_modification;
pub mod character_progression;
pub mod character_promotion;
pub mod character_revision_sequencer;
pub mod character_stance;
pub mod character_stash;
pub mod character_tutorial;
pub mod character_wheel;
pub mod charm_state;
pub mod chat_text;
pub mod content_activation;
mod db;
pub mod equipment_policy_abi;
mod field_capability_port;
pub mod forge;
pub mod fresh_admission;
pub mod fresh_admission_composition;
pub mod ground_cell_limit;
pub mod guild;
pub mod guild_audit;
pub mod guild_bank;
mod guild_bank_audit;
pub mod highscore_snapshot;
pub mod house_disposition;
pub mod house_interior;
pub mod house_interior_audit;
pub mod house_ownership;
mod house_ownership_audit;
pub mod house_scope_handoff;
pub mod house_spell_acl;
pub mod imbuement;
pub mod item_craft;
pub mod item_decay_retire;
pub mod item_decay_retire_audit;
pub mod item_fee_burn;
pub mod item_fee_burn_audit;
pub mod item_imbuement;
pub mod item_loop_move;
pub mod item_mint;
pub mod item_mint_audit;
pub mod item_text;
pub mod item_timed_repair;
pub mod item_timed_runtime;
pub mod item_timed_state;
pub mod item_timed_state_audit;
pub mod item_transfer;
pub mod item_transfer_audit;
pub mod item_use;
pub mod item_use_audit;
#[cfg(any(test, feature = "test-support"))]
pub mod item_weight_lookup;
pub mod loop_training;
pub mod mail;
pub mod map_item_mint;
pub mod map_item_mint_audit;
pub mod market;
pub mod monk_state;
pub mod native_admission_source;
pub mod native_map_items;
pub mod native_map_items_abi;
pub mod newhaven_travel;
pub mod npc_purchase;
pub mod npc_trade;
pub mod npc_trade_audit;
pub mod npc_travel;
pub mod offline_training_state;
pub mod party_pz_fact;
pub mod player_trade;
pub mod premium_activation_latch;
pub mod premium_fence;
pub mod promotion_purchase;
pub mod public_projections;
pub mod pvp_death_consequences;
mod pvp_death_request;
mod pvp_expiry;
pub mod pvp_live;
mod pvp_skull_storage;
pub mod pvp_state;
pub mod quest_state;
pub mod recovery_evidence_composition;
pub mod reward_claim_mint;
pub mod reward_claim_mint_audit;
pub mod runtime_scope_assignment;
mod schema;
pub mod spell_character_lookup;
pub mod spell_familiar_group;
pub mod spell_field_policy;
pub mod spell_house_abi;
pub mod spell_item_temporal;
pub mod spell_item_transaction;
pub mod spell_items_abi;
pub mod spell_owner_commit;
pub mod spell_parameter_result;
pub mod spell_privacy;
pub mod sqlstate_codes;
pub mod starter_grant;
pub mod tutorial_corruptor;
mod tutorial_death;
pub mod tutorial_islet_placement;
pub mod world_event;
pub mod world_house_instance;
pub mod world_party;
pub mod world_reset;
// End sorted module declarations.

pub use admission_journal::AdmissionReconnectJournal;
pub use db::{DB_PASS_DEADLINE, DurabilityRoot, DurabilityRootConfig};
pub use schema::{MigrationExecutor, SchemaCompatibility};

#[cfg(test)]
include!("core_modules_linkage_tests.rs");

use crate::foundation::{
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
    /// A database-raised error, with its registered SQLSTATE kind when it has one; the payload
    /// itself is never kept.
    Database(Option<sqlstate_codes::SqlstateKind>),
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
            sqlx::Error::Database(database) => Self::Database(
                database
                    .code()
                    .as_deref()
                    .and_then(sqlstate_codes::SqlstateKind::from_sqlstate),
            ),
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
    SessionUseAuthorization(
        crate::foundation::admission_authority_publication::GameSessionUseAuthorizationErrorV1,
    ),
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
    /// The registered SQLSTATE kind the database raised, kept when the error was converted.
    #[must_use]
    pub fn sqlstate(&self) -> Option<sqlstate_codes::SqlstateKind> {
        match self {
            Self::Database(SqlxFailureClass::Database(kind))
            | Self::Migration(MigrationFailureClass::Execute(SqlxFailureClass::Database(kind))) => {
                *kind
            }
            _ => None,
        }
    }

    pub fn from_commit_error(error: sqlx::Error) -> Self {
        let class = SqlxFailureClass::classify(&error);
        drop(error);
        if matches!(class, SqlxFailureClass::Database(_)) {
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
#[path = "core_modules_wp3_error_classification_tests.rs"]
mod wp3_error_classification_tests;

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
    #[cfg(any(test, feature = "test-support"))]
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
    pub const fn legacy(&self) -> &AdmissionReconnectJournal {
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
        transition: &crate::foundation::admission_authority_publication::TerminalReplacementClaimTransitionV1,
        membership: crate::foundation::admission_authority_publication::GameSessionUseRequestV1,
    ) -> Result<ReconnectPrepareDispositionV2, DurabilityError> {
        fresh_admission::encode_lifecycle(transition.evidence())?;
        self.prepare_owned(request, Some((transition.clone(), membership)))
            .await
    }

    async fn prepare_owned(
        &self,
        request: &ReconnectPrepareRequestV2,
        claims: Option<(crate::foundation::admission_authority_publication::TerminalReplacementClaimTransitionV1, crate::foundation::admission_authority_publication::GameSessionUseRequestV1)>,
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
        claims: Option<(crate::foundation::admission_authority_publication::TerminalReplacementClaimTransitionV1, crate::foundation::admission_authority_publication::GameSessionUseRequestV1)>,
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

fn encode_evidence_v2(evidence: &crate::foundation::AuthorityEvidenceFenceV1) -> serde_json::Value {
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
#[path = "core_modules_terminal_replacement_schema_red_tests.rs"]
mod terminal_replacement_schema_red_tests;

#[cfg(test)]
#[allow(clippy::expect_used, clippy::too_many_arguments)]
#[path = "core_modules_terminal_replacement_foundation_red_tests.rs"]
mod terminal_replacement_foundation_red_tests;
