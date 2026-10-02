//! Native Oteryn Game Server composition root.
//!
//! Foundation and the protocol/runtime/admission seam are merged. Domain semantics are composed
//! here while executable gameplay remains fail-closed until the later integration gates.

extern crate self as oteryn_game_server;

#[allow(dead_code)]
#[allow(clippy::duplicate_mod)] // Foundation's standalone tests include Ability source.
mod ability;
// AI-3 (#162; GAME-AI-01-ACTION-INTEGRATION-FIRST-CREATURE-SLICE-V1 §4.4/§5): compiles the
// bootstrap AI module into the server for the first time (previously only `tests/ai_bootstrap.rs`
// built it standalone).
#[allow(
    dead_code,
    unused_imports,
    reason = "ai:: has no production caller yet; a later wiring task consumes it from ai_think"
)]
mod ai;
// AI-3's composition root over `ai::`'s public API: builds one creature's per-think decision and,
// for chase/wander, acts through the real Movement owner. A sibling of `ai`, not a child of it,
// so `tests/ai_bootstrap.rs`'s standalone path-include of `ai/mod.rs` never pulls in
// `foundation`/`content`/`movement` (see `ai_think.rs`'s module doc). No production caller wires
// the composed per-creature think into `ChannelRuntimeV1`'s live owner cycle yet -- same pattern
// as AI-1's `owner_timer` and AI-2's spawn/respawn code: complete, tested and uncalled, ready for
// that wiring.
mod ai_monster_melee;
#[allow(
    dead_code,
    reason = "ai_think has no production caller yet; AI-4/a later wiring task composes it into ChannelRuntimeV1's owner cycle"
)]
mod ai_think;
#[allow(
    dead_code,
    reason = "nonshipping fixed-one-creature Combat proof awaits production composition"
)]
mod combat;
// A top-level module, not a submodule of `combat`: `combat.rs` is also recompiled standalone by
// `foundation/mod.rs`'s `#[cfg(test)] #[path = "../combat.rs"] mod exact_actor_test_combat;` and
// by several PG test binaries that need `combat::death_reward`/`combat::loot_plan` but not
// Content, so a `crate::content` dependency inside `combat.rs`'s own module tree would force
// every one of those to also carry a local `content` module. `combat_pickup` needs
// `crate::content` (B3-2: definition facts bound to the current Content generation) and reaches
// `combat::death_reward::DurabilitySession` through `combat`'s own `pub(crate)` re-export
// instead.
#[allow(
    dead_code,
    reason = "B3-2 has no production caller yet; a later admission stage wires one"
)]
#[path = "combat/pickup.rs"]
mod combat_pickup;
// D39 chest `USE` wiring, top-level for the same reason: `tests/interaction_workflow.rs`
// recompiles `interaction/mod.rs` without Content or durability. C2 routes `USE_INTENT` to it.
#[path = "interaction/chest_use.rs"]
mod interaction_chest_use;
// ACHIEVEMENT: the runtime Achievement catalogue, loaded with the Content activation and read by
// the chest `USE`.
mod achievement_catalogue;

pub mod character_bootstrap_intent;
pub mod character_recovery_fence;
#[allow(
    dead_code,
    unused_imports,
    reason = "CHAT-1a local chat rules await the CHAT_V1 wire and runtime of CHAT-1b"
)]
mod chat;
pub mod content;
pub mod domain;
pub mod durability;
pub mod foundation;
mod gameplay_transport;
#[allow(
    dead_code,
    reason = "unactivated local Movement proof awaits the separate production composition lease"
)]
mod movement;
pub mod node;
#[allow(
    dead_code,
    reason = "player spell core (P3a) awaits the cast protocol and runtime state contracts (P3b)"
)]
mod spell;

pub use gameplay_transport::{
    FreshEvidenceSource, GameplayListenerConfig, GameplaySeamOwners, GameplayServeError,
    serve_gameplay,
};

#[allow(
    dead_code,
    unused_imports,
    reason = "GAME-INTERACTION identity; the §7 revert driver is its first library consumer"
)]
mod interaction;
#[allow(
    dead_code,
    reason = "§7 timed-revert driver awaits its live Channel/Instance scope owner"
)]
mod world_object_revert;
#[allow(dead_code)]
pub(crate) mod world_runtime;

#[cfg(test)]
#[path = "../tests/support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;

#[cfg(test)]
#[path = "../tests/support/spell_stance_runtime_postgres_cases.rs"]
mod spell_stance_runtime_postgres_cases;

#[cfg(test)]
#[path = "../tests/support/spell_familiar_runtime_postgres_cases.rs"]
mod spell_familiar_runtime_postgres_cases;

#[cfg(test)]
#[path = "durability/world_party_tests.rs"]
mod world_party_runtime_postgres_cases;

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
mod v2_reconciled_prepared_budget_regression_tests {
    use super::foundation::{
        AccountPresenceClaimV1, AuthenticatedTransportRefV1, AuthorityEvidenceFenceV1, ChannelId,
        CharacterId, CharacterLease, CharacterWorldEligibilityClaimV1, CommandId,
        ConnectionGeneration, ControlLossEpochRefV1, Fnd02ReconciliationFenceV1,
        FreshAdmissionCommit, FreshAdmissionFacts, GameSessionAuthoritySnapshot, GameSessionId,
        GameSessionState, ProtectionEntitlementV1, ReconnectAttemptBudgetV1, ReconnectAttemptRef,
        ReconnectAttemptReservationV1, ReconnectAuthorityFenceV1, ReconnectCandidateBindingV1,
        ReconnectCompatibilityEvidenceV1, ReconnectConnectionFenceV1, ReconnectContinuityV1,
        ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1, ReconnectDurabilityFlowV1,
        ReconnectDurabilityFlowV2, ReconnectDurabilityRecordV1, ReconnectDurableOutcomeV2,
        ReconnectDurableReconciliationSnapshotV2, ReconnectIdentityV1, ReconnectPrepareActionV1,
        ReconnectPrepareActionV2, ReconnectPrepareCompletionV1, ReconnectPrepareCompletionV2,
        ReconnectPrepareDispositionV1, ReconnectPrepareDispositionV2,
        ReconnectProjectionDecisionV2, ReconnectProofV1, RuntimeScopeRefV1,
        ScopeOwnershipGeneration, TerminalGameSessionReplacementAuthorizationV1, WorldId,
    };

    fn invalid_record<E>(_error: E) -> ReconnectDurabilityErrorV1 {
        ReconnectDurabilityErrorV1::InvalidRecord
    }

    fn uuid_v7(raw: u64) -> [u8; 16] {
        let mut out = [0_u8; 16];
        out[8..].copy_from_slice(&raw.to_be_bytes());
        out[6] = 0x70;
        out[8] = (out[8] & 0x3f) | 0x80;
        out
    }

    fn sample_record() -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        let game_session_id = GameSessionId::decode(&uuid_v7(10)).map_err(invalid_record)?;
        let character_id = CharacterId::decode(&uuid_v7(11)).map_err(invalid_record)?;
        let world_id = WorldId::decode(&uuid_v7(12)).map_err(invalid_record)?;
        let channel_id = ChannelId::decode(&uuid_v7(13)).map_err(invalid_record)?;
        let identity = ReconnectIdentityV1::new(
            game_session_id,
            ReconnectAttemptRef::new(1).map_err(invalid_record)?,
            "123e4567-e89b-12d3-a456-426614174000",
            character_id,
            world_id,
            RuntimeScopeRefV1::channel(world_id, channel_id),
        )?;
        let connection = ReconnectConnectionFenceV1::new(
            ConnectionGeneration::new(7).map_err(invalid_record)?,
            ConnectionGeneration::new(8).map_err(invalid_record)?,
            AuthenticatedTransportRefV1::decode(&[1_u8; 16]).map_err(invalid_record)?,
        )?;
        let authority = ReconnectAuthorityFenceV1::new(
            9,
            ScopeOwnershipGeneration::new(10).map_err(invalid_record)?,
        )?;
        let continuity = ReconnectContinuityV1::new(
            ControlLossEpochRefV1::new(3)?,
            120,
            115,
            ProtectionEntitlementV1::unused(),
        )?;
        let fnd02 = Fnd02ReconciliationFenceV1::new(
            CommandId::new(1).map_err(invalid_record)?,
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

    fn terminal_predecessor_snapshot()
    -> Result<GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>, ReconnectDurabilityErrorV1>
    {
        let character_id = CharacterId::decode(&uuid_v7(11)).map_err(invalid_record)?;
        let world_id = WorldId::decode(&uuid_v7(12)).map_err(invalid_record)?;
        let channel_id = ChannelId::decode(&uuid_v7(13)).map_err(invalid_record)?;
        let facts = FreshAdmissionFacts::new([0x44; 32], character_id, world_id, channel_id, 9, 10)
            .map_err(invalid_record)?;
        let initial_transport =
            AuthenticatedTransportRefV1::decode(&[0x70; 16]).map_err(invalid_record)?;
        let commit = FreshAdmissionCommit::from_facts(
            GameSessionId::decode(&uuid_v7(20)).map_err(invalid_record)?,
            facts,
            initial_transport,
        )
        .map_err(invalid_record)?;
        GameSessionAuthoritySnapshot::from_current_facts(
            commit,
            GameSessionState::Terminal,
            ConnectionGeneration::new(7).map_err(invalid_record)?,
            None,
            CharacterLease::new(character_id, 9).map_err(invalid_record)?,
            Some(CharacterWorldEligibilityClaimV1::new(
                character_id,
                world_id,
            )),
            RuntimeScopeRefV1::channel(world_id, channel_id),
            ScopeOwnershipGeneration::new(10).map_err(invalid_record)?,
        )?
        .with_control_loss_continuity(ControlLossEpochRefV1::new(3)?, 120)
    }

    #[test]
    fn reconciled_prepared_marks_the_attempt_prepared_in_the_local_budget()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record()?;
        let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
        assert_eq!(
            budget.reserve(
                record.identity().reconnect_attempt_ref(),
                record.connection().transport_ref(),
            )?,
            ReconnectAttemptReservationV1::New
        );
        let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
        assert_eq!(
            flow.accept_prepare_completion(
                ReconnectPrepareCompletionV2::for_request(
                    &request,
                    ReconnectPrepareDispositionV2::Ambiguous,
                ),
                &mut budget,
            )?,
            ReconnectPrepareActionV2::ReconcileSameAttempt
        );
        assert_eq!(
            flow.accept_reconciliation(
                ReconnectDurableReconciliationSnapshotV2::new(
                    record.clone(),
                    ReconnectDurableOutcomeV2::Prepared,
                ),
                ReconnectCurrentAuthorityV1::from_record(&record, 105)?,
                &mut budget,
            )?,
            ReconnectProjectionDecisionV2::AwaitFinalRevalidation
        );

        let second_attempt = ReconnectAttemptRef::new(2).map_err(invalid_record)?;
        let second_transport =
            AuthenticatedTransportRefV1::decode(&[2_u8; 16]).map_err(invalid_record)?;
        assert_eq!(
            budget.reserve(second_attempt, second_transport)?,
            ReconnectAttemptReservationV1::New
        );
        assert_eq!(
            budget.accept_prepare_completion(
                second_attempt,
                second_transport,
                ReconnectPrepareDispositionV1::Prepared,
            ),
            Err(ReconnectDurabilityErrorV1::ConcurrentPrepared)
        );
        Ok(())
    }

    #[test]
    fn final_revalidation_rejects_same_world_different_runtime_scope()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record()?;
        let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
        budget.reserve(
            record.identity().reconnect_attempt_ref(),
            record.connection().transport_ref(),
        )?;
        let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
        assert_eq!(
            flow.accept_prepare_completion(
                ReconnectPrepareCompletionV2::for_request(
                    &request,
                    ReconnectPrepareDispositionV2::Prepared,
                ),
                &mut budget,
            )?,
            ReconnectPrepareActionV2::AwaitFinalRevalidation
        );

        let other_channel = ChannelId::decode(&uuid_v7(14)).map_err(invalid_record)?;
        let current = ReconnectCurrentAuthorityV1::from_current_facts(
            &record,
            Some(AccountPresenceClaimV1::from_identity(record.identity())?),
            Some(CharacterWorldEligibilityClaimV1::from_identity(
                record.identity(),
            )),
            Some(ReconnectCandidateBindingV1::from_record(&record)?),
            RuntimeScopeRefV1::channel(record.identity().world_id(), other_channel),
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
        )?;
        assert_eq!(
            flow.authorize_commit(current, 104),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );
        Ok(())
    }

    #[test]
    fn final_revalidation_rejects_authority_observed_after_authorization_deadline()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record()?;
        let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
        budget.reserve(
            record.identity().reconnect_attempt_ref(),
            record.connection().transport_ref(),
        )?;
        let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
        assert_eq!(
            flow.accept_prepare_completion(
                ReconnectPrepareCompletionV2::for_request(
                    &request,
                    ReconnectPrepareDispositionV2::Prepared,
                ),
                &mut budget,
            )?,
            ReconnectPrepareActionV2::AwaitFinalRevalidation
        );

        let current = ReconnectCurrentAuthorityV1::from_current_facts(
            &record,
            Some(AccountPresenceClaimV1::from_identity(record.identity())?),
            Some(CharacterWorldEligibilityClaimV1::from_identity(
                record.identity(),
            )),
            Some(ReconnectCandidateBindingV1::from_record(&record)?),
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
            106,
        )?;
        assert_eq!(
            flow.authorize_commit(current, 104),
            Err(ReconnectDurabilityErrorV1::DeadlineExpired)
        );
        Ok(())
    }

    #[test]
    fn v1_final_revalidation_rejects_authority_observed_after_authorization_deadline()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record()?;
        let (mut flow, request) = ReconnectDurabilityFlowV1::begin(record.clone());
        assert_eq!(
            flow.accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                &request,
                ReconnectPrepareDispositionV1::Prepared,
            ))?,
            ReconnectPrepareActionV1::AwaitFinalRevalidation
        );
        let current = ReconnectCurrentAuthorityV1::from_current_facts(
            &record,
            Some(AccountPresenceClaimV1::from_identity(record.identity())?),
            Some(CharacterWorldEligibilityClaimV1::from_identity(
                record.identity(),
            )),
            Some(ReconnectCandidateBindingV1::from_record(&record)?),
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
            106,
        )?;
        assert_eq!(
            flow.authorize_commit(current, 104),
            Err(ReconnectDurabilityErrorV1::DeadlineExpired)
        );
        Ok(())
    }

    #[test]
    fn terminal_replacement_authorization_requires_current_account_presence_at_prepare()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let record = sample_record()?;
        let predecessor_session_id = GameSessionId::decode(&uuid_v7(20)).map_err(invalid_record)?;
        let exact_presence = AccountPresenceClaimV1::from_identity(record.identity())?;
        let reassigned_presence = AccountPresenceClaimV1::new(
            record.identity().account_id(),
            CharacterId::decode(&uuid_v7(99)).map_err(invalid_record)?,
        )?;

        assert!(
            TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
                record.identity().account_id(),
                Some(&exact_presence),
                predecessor_session_id,
                record.identity().game_session_id(),
                terminal_predecessor_snapshot()?,
                &record,
            )
            .is_ok()
        );
        assert_eq!(
            TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
                record.identity().account_id(),
                None,
                predecessor_session_id,
                record.identity().game_session_id(),
                terminal_predecessor_snapshot()?,
                &record,
            ),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );
        assert_eq!(
            TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
                record.identity().account_id(),
                Some(&reassigned_presence),
                predecessor_session_id,
                record.identity().game_session_id(),
                terminal_predecessor_snapshot()?,
                &record,
            ),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );
        Ok(())
    }
}

pub mod admission_evidence;
pub mod native_admission_source;
