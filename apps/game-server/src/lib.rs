//! Native Oteryn Game Server composition root.
//!
//! Foundation and the protocol/runtime/admission seam are merged. Domain semantics are composed
//! here while executable gameplay remains fail-closed until the later integration gates.

extern crate self as oteryn_game_server;

// Keep the former module paths available to native composition and qualification consumers.

#[allow(dead_code)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_combat::ability;
// Native source combat requires the actual Channel/Condition owners; it is
// intentionally outside the standalone generic Ability fixture facade.
#[allow(dead_code)] // Preserve the original Ability module lint scope for this native-only adapter.
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_combat::player_lethal;
// AI-3 (#162; GAME-AI-01-ACTION-INTEGRATION-FIRST-CREATURE-SLICE-V1 §4.4/§5): compiles the
// bootstrap AI module into the server for the first time (previously only `tests/ai_bootstrap.rs`
// built it standalone).
#[allow(
    dead_code,
    unused_imports,
    reason = "ai:: has no production caller yet; a later wiring task consumes it from ai_think"
)]
use oteryn_gameplay_monster_ai::ai;
// AI-3's composition root over `ai::`'s public API: builds one creature's per-think decision and,
// for chase/wander, acts through the real Movement owner. A sibling of `ai`, not a child of it,
// so `tests/ai_bootstrap.rs`'s standalone path-include of `ai/mod.rs` never pulls in
// `foundation`/`content`/`movement` (see `ai_think.rs`'s module doc). No production caller wires
// the composed per-creature think into `ChannelRuntimeV1`'s live owner cycle yet -- same pattern
// as AI-1's `owner_timer` and AI-2's spawn/respawn code: complete, tested and uncalled, ready for
// that wiring.
#[allow(
    dead_code,
    reason = "nonshipping fixed-one-creature Combat proof awaits production composition"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_combat::combat;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_monster_ai::ai_monster_melee;
#[allow(
    dead_code,
    reason = "ai_think has no production caller yet; AI-4/a later wiring task composes it into ChannelRuntimeV1's owner cycle"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_monster_ai::ai_think;
// A top-level module, not a submodule of `combat`: `combat.rs` is also recompiled standalone by
// several PG test binaries that need `combat::death_reward`/`combat::loot_plan` but not
// Content, so a `crate::content` dependency inside `combat.rs`'s own module tree would force
// every one of those to also carry a local `content` module. `combat_pickup` needs
// `crate::content` (B3-2: definition facts bound to the current Content generation) and reaches
// `combat::death_reward::DurabilitySession` through `combat`'s own `pub(crate)` re-export
// instead.
#[allow(
    dead_code,
    reason = "B3-2 has no production caller yet; a later admission stage wires one"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_combat::combat_pickup;
// ATTACK-1a, top-level for the same reason as `combat_pickup`: it evaluates its formulas with
// `crate::spell::formula`, which the standalone `combat.rs` inclusions do not carry.
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_combat::combat_attack;
// PVP-RT-1 (pure half): PvP legality and combat lock; not wired until the PVP-0 §8 go-live gate.
#[allow(
    dead_code,
    reason = "PvP stays off until the PVP-0 go-live gate; PVP-RT-1b wires the legality stage"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_combat::combat_pvp;
// PARTY-1 (CP D1391): the party module and its PostgreSQL cases.
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_social::party;
#[cfg(test)]
#[path = "lifted_tests/party_shared_xp_postgres_tests.rs"]
mod party_shared_xp_postgres_tests;
// D39 chest `USE` wiring, top-level for the same reason: `tests/interaction_workflow.rs`
// recompiles `interaction/mod.rs` without Content or durability. C2 routes `USE_INTENT` to it.
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_social::interaction_chest_use;
// ACHIEVEMENT: the runtime Achievement catalogue, loaded with the Content activation and read by
// the chest `USE`.
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_social::achievement_catalogue;

pub use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
pub use oteryn_gameplay_content::content;
use oteryn_gameplay_dispatch::gameplay_transport;
pub use oteryn_gameplay_domain::domain;
pub use oteryn_gameplay_durability::character_bootstrap_intent;
pub use oteryn_gameplay_durability::durability;
pub use oteryn_gameplay_entities as foundation;
#[allow(
    dead_code,
    unused_imports,
    reason = "CHAT-1a local chat rules await the CHAT_V1 wire and runtime of CHAT-1b"
)]
use oteryn_gameplay_social::chat;
pub use oteryn_gameplay_world::house_scope;
pub use oteryn_gameplay_world::map;
#[allow(
    dead_code,
    reason = "unactivated local Movement proof awaits the separate production composition lease"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_world::movement;
pub mod node;
pub use oteryn_gameplay_progression::wheel_gem_data;
pub use oteryn_gameplay_social::premium;
#[allow(
    dead_code,
    reason = "player spell core (P3a) awaits the cast protocol and runtime state contracts (P3b)"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_spell::spell;

pub use gameplay_transport::{
    FreshEvidenceSource, GameplayListenerConfig, GameplaySeamOwners, GameplayServeError,
    serve_gameplay,
};

#[allow(
    dead_code,
    unused_imports,
    reason = "GAME-INTERACTION identity; the §7 revert driver is its first library consumer"
)]
use oteryn_gameplay_domain::domain::interaction;
#[allow(
    dead_code,
    reason = "§7 timed-revert driver awaits its live Channel/Instance scope owner"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_world::world_object_revert;
#[allow(dead_code)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_world::world_runtime;

#[cfg(test)]
#[allow(
    dead_code,
    reason = "shared PostgreSQL support; each mount uses a subset"
)]
#[path = "../tests/support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;

#[cfg(test)]
#[path = "../tests/support/spell_stance_runtime_postgres_cases.rs"]
mod spell_stance_runtime_postgres_cases;

#[cfg(test)]
#[path = "../tests/support/spell_familiar_runtime_postgres_cases.rs"]
mod spell_familiar_runtime_postgres_cases;

// LIMIT_RAISE value/audit regressions have one source mount; the canonical PG root
// path-loads durability and must not collect these pure cases a second time.
#[cfg(test)]
#[path = "../../../crates/gameplay/durability/src/durability/forge_dust_limit_audit_tests.rs"]
mod forge_dust_limit_audit_tests;

// RANK-1 ranking order has one source mount; the PG roots path-load durability (which owns the
// ranking leaf) and must not collect these pure cases a second time.
#[cfg(test)]
#[path = "highscores/ranking_tests.rs"]
mod highscore_ranking_tests;

#[cfg(test)]
#[path = "lifted_tests/foundation_recovery_tests.rs"]
mod foundation_recovery_tests;

#[cfg(test)]
#[path = "lifted_tests/foundation_final_review_regressions.rs"]
mod foundation_final_review_regressions;

#[cfg(test)]
#[path = "lifted_tests/foundation_final_review_round2_regressions.rs"]
mod foundation_final_review_round2_regressions;

#[cfg(test)]
#[path = "lifted_tests/foundation_final_review_round2_runtime_rollback.rs"]
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

pub use oteryn_gameplay_admission_evidence as admission_evidence;
pub use oteryn_gameplay_native_admission_source as native_admission_source;

#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_combat::creature_appearance_content;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_combat::creature_area_heal;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_combat::creature_auto_attack;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_combat::creature_chain_attack;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_combat::creature_condition_content;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_combat::creature_defense_presentation;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_content::content::creature_attack_geometry;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_encounters::bosses::bone_overlord::bone_phase_registry;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_encounters::bosses::crystal_deaths::crystal_callbacks;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_encounters::bosses::crystal_deaths::crystal_death_composition;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_encounters::bosses::herald_of_fire::crystal_herald_quest;
#[allow(dead_code)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_encounters::bosses::smelly_cheese::smelly_encounter_dispatch;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_encounters::bosses::smelly_cheese::timers as smelly_cheese;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_intent;
#[allow(dead_code)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_spawn;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_encounters::self_heal_qualification;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_spell::creature_damage_spell;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_spell::creature_damage_spell_delayed;
// Source-bound native library callbacks; shipping server activation remains separate.
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_encounters::bosses::crystal_deaths::crystal_death_composition::speech as weak_spot_speech;
#[allow(
    dead_code,
    reason = "Private native encounter owner ABI retained for source-qualified library dispatch; shipping AI loop is not activated"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_encounters::bosses::seven_encounters::source_encounter_seven;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_encounters::bosses::smelly_cheese::smelly_damage;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports)]
use oteryn_gameplay_encounters::bosses::the_welter::welter_consume;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_encounters::bosses::threshold_heals::source_threshold_heal;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_source_critical_baseline as source_critical_baseline;
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_weather as world_weather;
#[allow(
    dead_code,
    reason = "LOOP-CLOCK-1 registers the pure World clock; the WORLD_LIGHT_V1 offer composes it later"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_world::world_clock;

#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_encounters::bosses::native_combat::monster_combat_lane;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_monster_ai::monster_owner_cycle;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_monster_ai::monster_summon;

#[allow(dead_code)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_encounters::bosses::icicle::creature_icicle;

#[allow(unused_imports, reason = "preserve the former server module path")]
use durability::{native_world_items, native_world_read};

#[allow(unused_imports, reason = "preserve the former server module path")]
use durability::creature_source_items;
#[allow(unused_imports, reason = "preserve the former server module path")]
use durability::mail_text_policy;

#[cfg(test)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_social::security;

#[cfg(test)]
#[path = "lifted_tests/domain_death_tests.rs"]
mod domain_death_tests;
#[cfg(test)]
#[path = "../../../crates/gameplay/durability/src/durability/forge_tests.rs"]
mod domain_forge_tests;
#[cfg(test)]
#[path = "lifted_tests/domain_market_tests.rs"]
mod domain_market_tests;
#[cfg(test)]
#[path = "lifted_tests/domain_timed_item_host_tests.rs"]
mod domain_timed_item_host_tests;

#[cfg(test)]
#[path = "lifted_tests/content_executable_catalog_tests.rs"]
mod content_executable_catalog_tests;

#[cfg(test)]
#[path = "lifted_tests/content_native_spell_tests.rs"]
mod content_native_spell_tests;

#[cfg(test)]
#[path = "lifted_tests/content_encounter_map_item_tests.rs"]
mod content_encounter_map_item_tests;

#[cfg(test)]
#[path = "lifted_tests/content_creature_reward_tests.rs"]
mod content_creature_reward_tests;

#[cfg(test)]
#[path = "lifted_tests/content_charm_source_effect_tests.rs"]
mod content_charm_source_effect_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "lifted_tests/foundation_ability_exact_actor_resolution_tests.rs"]
mod foundation_ability_exact_actor_resolution_tests;
#[cfg(test)]
#[path = "lifted_tests/world_clock_world_config_tests.rs"]
mod world_clock_world_config_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "lifted_tests/foundation_channel_owner_ability_commit_tests.rs"]
mod foundation_channel_owner_ability_commit_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "lifted_tests/foundation_channel_owner_creature_bite_tests.rs"]
mod foundation_channel_owner_creature_bite_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "lifted_tests/foundation_channel_owner_combat_death_tests.rs"]
mod foundation_channel_owner_combat_death_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "lifted_tests/foundation_movement_static_kernel_structural_tests.rs"]
mod foundation_movement_static_kernel_structural_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "lifted_tests/foundation_attacker_fence_tests.rs"]
mod foundation_attacker_fence_tests;
// Higher-layer qualification tests stay in the native composition root.

#[cfg(test)]
#[path = "lifted_tests/creature_icicle_tests.rs"]
mod creature_icicle_tests;

#[cfg(test)]
#[path = "lifted_tests/creature_appearance_content_tests.rs"]
mod creature_appearance_content_tests;

#[cfg(test)]
#[path = "lifted_tests/creature_condition_content_tests.rs"]
mod creature_condition_content_tests;

#[cfg(test)]
#[path = "lifted_tests/creature_condition_content_gate_tests.rs"]
mod creature_condition_content_gate_tests;

#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
#[allow(unused_imports, reason = "preserve the former server module path")]
use oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast;

#[cfg(test)]
mod qualification;
#[cfg(test)]
mod quest_catalogue_boot_tests;

#[cfg(test)]
mod bundle_spawn_owner_tests;
