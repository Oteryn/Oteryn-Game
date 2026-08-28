//! Native Oteryn Game Server composition root.
//!
//! Foundation and the protocol/runtime/admission seam are merged. Domain semantics are composed
//! here while executable gameplay remains fail-closed until the later integration gates.

pub mod content;
pub mod domain;
pub mod foundation;

#[cfg(test)]
#[path = "foundation/recovery_tests.rs"]
mod foundation_recovery_tests;

#[cfg(test)]
#[path = "foundation/final_review_regressions.rs"]
mod foundation_final_review_regressions;

#[cfg(test)]
#[path = "foundation/final_review_round2_regressions.rs"]
mod foundation_final_review_round2_regressions;

#[cfg(test)]
#[path = "foundation/final_review_round2_runtime_rollback.rs"]
mod foundation_final_review_round2_runtime_rollback;

use oteryn_foundation::CancellationToken;
use oteryn_simulation_determinism::{SimulationDeterminismProfile, active_profile};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use tokio::runtime::Builder;

pub const GAMEPLAY_UNAVAILABLE_REASON: &str =
    "native gameplay transport and executable gameplay slices are not yet integrated";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayAvailability {
    UnavailableBootstrap,
}

#[derive(Debug, Clone)]
pub struct GameServerBootstrap {
    shutdown: CancellationToken,
    determinism_profile: SimulationDeterminismProfile,
}

impl Default for GameServerBootstrap {
    fn default() -> Self {
        Self::new()
    }
}

impl GameServerBootstrap {
    #[must_use]
    pub fn new() -> Self {
        Self {
            shutdown: CancellationToken::new(),
            determinism_profile: active_profile(),
        }
    }

    #[must_use]
    pub const fn gameplay_availability(&self) -> GameplayAvailability {
        GameplayAvailability::UnavailableBootstrap
    }

    #[must_use]
    pub const fn gameplay_unavailable_reason(&self) -> &'static str {
        GAMEPLAY_UNAVAILABLE_REASON
    }

    #[must_use]
    pub const fn determinism_profile(&self) -> SimulationDeterminismProfile {
        self.determinism_profile
    }

    pub fn request_shutdown(&self) {
        self.shutdown.cancel();
    }

    #[must_use]
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown.is_cancelled()
    }

    pub async fn run_until_shutdown(&self) {
        self.shutdown.cancelled().await;
    }
}

#[derive(Debug)]
pub enum BootstrapSmokeError {
    Runtime(std::io::Error),
    GameplayUnexpectedlyAvailable,
}

impl Display for BootstrapSmokeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(error) => write!(formatter, "cannot create bootstrap runtime: {error}"),
            Self::GameplayUnexpectedlyAvailable => {
                formatter.write_str("bootstrap unexpectedly reported gameplay availability")
            }
        }
    }
}

impl Error for BootstrapSmokeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Runtime(error) => Some(error),
            Self::GameplayUnexpectedlyAvailable => None,
        }
    }
}

pub fn bootstrap_smoke() -> Result<(), BootstrapSmokeError> {
    let server = GameServerBootstrap::new();
    if server.gameplay_availability() != GameplayAvailability::UnavailableBootstrap {
        return Err(BootstrapSmokeError::GameplayUnexpectedlyAvailable);
    }

    server.request_shutdown();
    let runtime = Builder::new_current_thread()
        .build()
        .map_err(BootstrapSmokeError::Runtime)?;
    runtime.block_on(server.run_until_shutdown());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_simulation_determinism::SimulationDeterminismProfileRevision;

    #[test]
    fn bootstrap_is_explicitly_gameplay_unavailable() {
        let server = GameServerBootstrap::new();
        assert_eq!(
            server.gameplay_availability(),
            GameplayAvailability::UnavailableBootstrap
        );
        assert_eq!(
            server.gameplay_unavailable_reason(),
            GAMEPLAY_UNAVAILABLE_REASON
        );
        assert_eq!(
            server.determinism_profile().revision(),
            SimulationDeterminismProfileRevision::V1
        );
        assert!(!server.is_shutdown_requested());
    }

    #[test]
    fn shutdown_is_deterministic() -> Result<(), BootstrapSmokeError> {
        let server = GameServerBootstrap::new();
        server.request_shutdown();

        let runtime = Builder::new_current_thread()
            .build()
            .map_err(BootstrapSmokeError::Runtime)?;
        runtime.block_on(server.run_until_shutdown());

        assert!(server.is_shutdown_requested());
        Ok(())
    }

    #[test]
    fn content_evidence_seam_is_composed_but_ordinary_release_stays_closed()
    -> Result<(), Box<dyn Error>> {
        let limits = content::EvidenceLimits::new(
            "evidence:composition-smoke",
            262_144,
            8,
            131_072,
            256,
            4_096,
            128,
            256,
            256,
            64,
            1_024,
        )?;
        let source = content::synthetic_vsl_fixture(&limits)?;
        let compiled = content::compile(&source, &limits, content::CompileTarget::Evidence)?;
        assert!(!compiled.server_artifact.is_empty());
        assert!(matches!(
            content::compile(&source, &limits, content::CompileTarget::OrdinaryRelease),
            Err(content::ContentError::FixtureOnlyReleaseRejected)
        ));
        assert_eq!(
            GameServerBootstrap::new().gameplay_availability(),
            GameplayAvailability::UnavailableBootstrap
        );
        Ok(())
    }
    #[test]
    fn bootstrap_smoke_stays_fail_closed() -> Result<(), BootstrapSmokeError> {
        bootstrap_smoke()
    }
}

#[cfg(test)]
mod terminal_session_replacement_red_tests {
    use crate::foundation::{
        AuthenticatedTransportRefV1, AuthorityEvidenceFenceV1, ChannelId, CharacterId,
        CharacterLease, CommandId, ConnectionGeneration, ControlLossEpochRefV1,
        Fnd02ReconciliationFenceV1, FreshAdmissionCommit, FreshAdmissionFacts,
        GameSessionAuthoritySnapshot, GameSessionId, GameSessionState, PendingCommandDispositionV1,
        PendingCommandReconciliationV1, ProtectionEntitlementV1, ReconnectAttemptBudgetV1,
        ReconnectAttemptRef, ReconnectAttemptReservationV1, ReconnectAuthorityFenceV1,
        ReconnectCompatibilityEvidenceV1, ReconnectConnectionFenceV1, ReconnectContinuityV1,
        ReconnectDurabilityErrorV1, ReconnectDurabilityFlowV1, ReconnectDurabilityRecordV1,
        ReconnectDurabilityPhaseV1, ReconnectDurableOutcomeV2,
        ReconnectDurableReconciliationSnapshotV2, ReconnectDurableTerminalDispositionV1,
        ReconnectIdentityV1, ReconnectPrepareActionV1, ReconnectPrepareCompletionV1,
        ReconnectPrepareCompletionV2, ReconnectPrepareDispositionV1, ReconnectPrepareDispositionV2,
        ReconnectDurabilityFlowV2, ReconnectProofV1, RuntimeScopeRefV1, ScopeOwnershipGeneration,
        StateDomainRevisionV1, TerminalGameSessionReplacementAuthorizationV1, WorldId,
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
            ReconnectAttemptRef::new(attempt).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
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
            vec![StateDomainRevisionV1::new(1, 4)?, StateDomainRevisionV1::new(2, 7)?],
        )?;
        let platform = AuthorityEvidenceFenceV1::new(
            "platform-security", "reconnect", "account", "sec:17", "decision:sec:17", 100,
        )?;
        let trust = AuthorityEvidenceFenceV1::new(
            "proof-trust", "reconnect", "recovery-key", "trust:21", "decision:trust:21", 101,
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
        assert!(authorize(predecessor_snapshot(GameSessionState::Active, Some(AuthenticatedTransportRefV1::decode(&[0x70; 16]).expect("transport")), 11).expect("snapshot"), &candidate, predecessor, candidate_id).is_err());
        assert!(authorize(predecessor_snapshot(GameSessionState::Reconnectable, None, 11).expect("snapshot"), &candidate, predecessor, candidate_id).is_err());
        assert!(authorize(predecessor_snapshot(GameSessionState::Terminal, Some(AuthenticatedTransportRefV1::decode(&[0x70; 16]).expect("transport")), 11).expect("snapshot"), &candidate, predecessor, candidate_id).is_err());
        assert!(authorize(predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"), &candidate, predecessor, candidate_id).is_ok());
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
        assert_eq!(authorization.predecessor_current_scope_ownership_generation().get(), 11);
    }

    #[test]
    fn terminal_replacement_authorization_rejects_predecessor_session_mismatch() {
        let candidate = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 11, 1).expect("candidate");
        assert!(authorize(predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"), &candidate, game_session(99).expect("wrong predecessor"), game_session(20).expect("candidate")).is_err());
    }

    #[test]
    fn terminal_replacement_authorization_rejects_predecessor_connection_generation_mismatch() {
        let candidate = candidate_record(20, ACCOUNT, 11, 12, 6, 9, 11, 1).expect("candidate");
        assert!(authorize(predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"), &candidate, game_session(10).expect("predecessor"), game_session(20).expect("candidate")).is_err());
    }

    #[test]
    fn terminal_replacement_authorization_rejects_predecessor_lease_generation_mismatch() {
        let candidate = candidate_record(20, ACCOUNT, 11, 12, 7, 10, 11, 1).expect("candidate");
        assert!(authorize(predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"), &candidate, game_session(10).expect("predecessor"), game_session(20).expect("candidate")).is_err());
    }

    #[test]
    fn terminal_replacement_authorization_rejects_candidate_session_mismatch() {
        let candidate = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 11, 1).expect("candidate");
        assert!(authorize(predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"), &candidate, game_session(10).expect("predecessor"), game_session(21).expect("wrong candidate")).is_err());
    }

    #[test]
    fn terminal_replacement_authorization_rejects_candidate_account_mismatch() {
        let candidate = candidate_record(20, OTHER_ACCOUNT, 11, 12, 7, 9, 11, 1).expect("candidate");
        assert!(authorize(predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"), &candidate, game_session(10).expect("predecessor"), game_session(20).expect("candidate")).is_err());
    }

    #[test]
    fn terminal_replacement_authorization_rejects_candidate_character_mismatch() {
        let candidate = candidate_record(20, ACCOUNT, 99, 12, 7, 9, 11, 1).expect("candidate");
        assert!(authorize(predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"), &candidate, game_session(10).expect("predecessor"), game_session(20).expect("candidate")).is_err());
    }

    #[test]
    fn terminal_replacement_authorization_rejects_candidate_world_mismatch() {
        let candidate = candidate_record(20, ACCOUNT, 11, 99, 7, 9, 11, 1).expect("candidate");
        assert!(authorize(predecessor_snapshot(GameSessionState::Terminal, None, 11).expect("snapshot"), &candidate, game_session(10).expect("predecessor"), game_session(20).expect("candidate")).is_err());
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
        assert_eq!(flow.phase(), ReconnectDurabilityPhaseV1::ReconciliationRequired);
    }

    #[test]
    fn v2_direct_existing_terminal_collision_marks_budget_and_respects_capacity() {
        let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
        let attempt = record.identity().reconnect_attempt_ref();
        let transport = record.connection().transport_ref();
        let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
        assert_eq!(budget.reserve(attempt, transport).expect("reserve"), ReconnectAttemptReservationV1::New);
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
            (ReconnectDurableTerminalDispositionV1::TransportRefCollision, true),
            (ReconnectDurableTerminalDispositionV1::ConcurrentPrepared, false),
            (ReconnectDurableTerminalDispositionV1::StaleAuthority, false),
        ] {
            let record = candidate_record(20, ACCOUNT, 11, 12, 7, 9, 10, 1).expect("record");
            let attempt = record.identity().reconnect_attempt_ref();
            let transport = record.connection().transport_ref();
            let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
            budget.reserve(attempt, transport).expect("reserve");
            let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
            flow.accept_prepare_completion(
                ReconnectPrepareCompletionV2::for_request(&request, ReconnectPrepareDispositionV2::Ambiguous),
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
            assert_eq!(budget.replacement_allowed_after_collision(attempt), allows_replacement);
        }
    }
}
