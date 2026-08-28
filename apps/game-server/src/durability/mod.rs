//! PostgreSQL-backed, journal-only reconnect durability boundary.
//!
//! This module deliberately owns persistence/classification only. Foundation
//! constructs and revalidates reconnect authority; the runtime must submit the
//! resulting request asynchronously and consume its completion as new input.

mod admission_journal;
mod db;
mod schema;

pub use admission_journal::AdmissionReconnectJournal;
pub use schema::{MigrationExecutor, SchemaCompatibility};

use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum DurabilityError {
    Database(sqlx::Error),
    Migration(sqlx::migrate::MigrateError),
    SchemaIncompatible(SchemaCompatibility),
    InvalidStoredState,
}

impl Display for DurabilityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => {
                write!(formatter, "PostgreSQL durability operation failed: {error}")
            }
            Self::Migration(error) => write!(formatter, "game migration operation failed: {error}"),
            Self::SchemaIncompatible(state) => {
                write!(
                    formatter,
                    "game durability schema is not runtime-compatible: {state:?}"
                )
            }
            Self::InvalidStoredState => {
                formatter.write_str("durability journal contains invalid state")
            }
        }
    }
}

impl std::error::Error for DurabilityError {}

impl From<sqlx::Error> for DurabilityError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<sqlx::migrate::MigrateError> for DurabilityError {
    fn from(error: sqlx::migrate::MigrateError) -> Self {
        Self::Migration(error)
    }
}

#[cfg(test)]
mod terminal_replacement_schema_red_tests {
    const MIGRATION: &str = include_str!("../../migrations/0001_admission_reconnect_journal.sql");

    #[test]
    fn terminal_replacement_forward_syncs_lagging_scope_fence_atomically() {
        assert!(
            MIGRATION.contains("session_state SMALLINT NOT NULL DEFAULT 1 CHECK (session_state BETWEEN 1 AND 3)"),
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
            MIGRATION.contains("session_state SMALLINT NOT NULL DEFAULT 1 CHECK (session_state BETWEEN 1 AND 3)"),
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
mod terminal_replacement_foundation_red_tests {
    use oteryn_game_server::foundation::{
        AuthenticatedTransportRefV1, AuthorityEvidenceFenceV1, ChannelId, CharacterId,
        CharacterLease, CommandId, ConnectionGeneration, ControlLossEpochRefV1,
        Fnd02ReconciliationFenceV1, FreshAdmissionCommit, FreshAdmissionFacts,
        GameSessionAuthoritySnapshot, GameSessionId, GameSessionState, PendingCommandDispositionV1,
        PendingCommandReconciliationV1, ProtectionEntitlementV1, ReconnectAttemptBudgetV1,
        ReconnectAttemptRef, ReconnectAttemptReservationV1, ReconnectAuthorityFenceV1,
        ReconnectCompatibilityEvidenceV1, ReconnectConnectionFenceV1, ReconnectContinuityV1,
        ReconnectDurabilityErrorV1, ReconnectDurabilityFlowV1, ReconnectDurabilityFlowV2,
        ReconnectDurabilityPhaseV1, ReconnectDurabilityRecordV1, ReconnectDurableOutcomeV2,
        ReconnectDurableReconciliationSnapshotV2, ReconnectDurableTerminalDispositionV1,
        ReconnectIdentityV1, ReconnectPrepareActionV1, ReconnectPrepareCompletionV1,
        ReconnectPrepareCompletionV2, ReconnectPrepareDispositionV1, ReconnectPrepareDispositionV2,
        ReconnectProofV1, RuntimeScopeRefV1, ScopeOwnershipGeneration, StateDomainRevisionV1,
        TerminalGameSessionReplacementAuthorizationV1, WorldId,
    };

    const ACCOUNT: &str = "123e4567-e89b-12d3-a456-426614174000";
    const OTHER_ACCOUNT: &str = "123e4567-e89b-12d3-a456-426614174001";

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

    fn predecessor_snapshot(
        state: GameSessionState,
        current_transport: Option<AuthenticatedTransportRefV1>,
        current_scope: u64,
    ) -> Result<GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>, ReconnectDurabilityErrorV1>
    {
        let facts = FreshAdmissionFacts::new(
            [0x44; 32],
            character(11)?,
            world(12)?,
            channel(13)?,
            9,
            10,
        )
        .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let initial_transport = AuthenticatedTransportRefV1::decode(&[0x70; 16])
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let commit = FreshAdmissionCommit::from_facts(game_session(10)?, facts, initial_transport)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        Ok(GameSessionAuthoritySnapshot::new(
            commit,
            state,
            ConnectionGeneration::new(7).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            current_transport,
            CharacterLease::new(character(11)?, 9)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            ScopeOwnershipGeneration::new(current_scope)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        ))
    }

    fn authorize(
        snapshot: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        candidate: &ReconnectDurabilityRecordV1,
        expected_predecessor: GameSessionId,
        expected_candidate: GameSessionId,
    ) -> Result<TerminalGameSessionReplacementAuthorizationV1, ReconnectDurabilityErrorV1> {
        TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
            ACCOUNT,
            expected_predecessor,
            expected_candidate,
            snapshot,
            candidate,
        )
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
                    Some(
                        AuthenticatedTransportRefV1::decode(&[0x70; 16]).expect("transport"),
                    ),
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
                    Some(
                        AuthenticatedTransportRefV1::decode(&[0x70; 16]).expect("transport"),
                    ),
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
        let candidate = candidate_record(20, OTHER_ACCOUNT, 11, 12, 7, 9, 11, 1).expect("candidate");
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
            let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
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
            let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
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
                    record.authority().scope_ownership_generation(),
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
