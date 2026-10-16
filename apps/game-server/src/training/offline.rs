//! OFFLINE0 §§3–6: transition plans and the native fenced marker transaction source.
//!
//! Raw proposed marker/time/context data never grants authority. The writer independently
//! resolves current recovery/gameplay fences and commits the same build row and marker receipt.
//! Actual FND clock/predecessor and pinned Content rate/percent participants, schema, registration
//! and callers remain explicit integration dependencies; this source alone is not installed.
#![allow(
    dead_code,
    reason = "offline owner stages and native marker integration await CP"
)]

use crate::domain::CharacterId;
use crate::domain::premium::PremiumStatus;
use crate::durability::character_build::{BuildFormula, DurableBuildState, SKILLS, relevel};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::{GameSessionId, RuntimeScopeRefV1};

const POOL_CAP_S: u32 = 43_200;
const OFFLINE_CAP_S: u64 = 21 * 24 * 60 * 60;
const SECOND_US: i64 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OfflineSkill {
    Fist,
    Club,
    Sword,
    Axe,
    Distance,
    MagicLevel,
}
impl OfflineSkill {
    fn native_index(self) -> Option<usize> {
        let key = match self {
            Self::Fist => "fist",
            Self::Club => "club",
            Self::Sword => "sword",
            Self::Axe => "axe",
            Self::Distance => "distance",
            Self::MagicLevel => return None,
        };
        SKILLS.iter().position(|value| *value == key)
    }
}
/// Proposed columns on the EXISTING native build row, not a parallel storage model. There is
/// deliberately no timestamp or second copy of magic/skill levels here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProposedOfflineColumns {
    pub(crate) stamina_minutes: u16,
    pub(crate) pool_s: u32,
    pub(crate) skill: Option<OfflineSkill>,
}
impl Default for ProposedOfflineColumns {
    fn default() -> Self {
        Self {
            stamina_minutes: 2520,
            pool_s: POOL_CAP_S,
            skill: None,
        }
    }
}
impl ProposedOfflineColumns {
    fn validate(self) -> Result<(), PlanError> {
        if self.stamina_minutes > 2520 || self.pool_s > POOL_CAP_S {
            Err(PlanError::InvalidColumns)
        } else {
            Ok(())
        }
    }
}
/// Private plan names only; these are NOT new native BuildCause values or receipt IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkerKind {
    Logout,
    Settlement,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MarkerKey {
    pub(crate) character: CharacterId,
    pub(crate) lease: u64,
    pub(crate) kind: MarkerKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProposedLookup {
    Missing,
    Existing(MarkerKey),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProposedPredecessor {
    pub(crate) generation: u64,
    pub(crate) session: GameSessionId,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProposedLeaseTimes {
    pub(crate) acquired_at_us: i64,
    pub(crate) session_created_at_us: i64,
    /// Independently resolved immediate predecessor relation, NOT inferred as generation-1.
    pub(crate) predecessor: Option<ProposedPredecessor>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProposedLogoutMarker {
    pub(crate) key: MarkerKey,
    pub(crate) session: GameSessionId,
    pub(crate) scope: RuntimeScopeRefV1,
    pub(crate) as_of_us: i64,
    pub(crate) committed_at_us: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LatestBuildReceipt {
    Absent,
    OtherBuildCarryingReceipt,
    Logout(ProposedLogoutMarker),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProgressionRow {
    Present,
    Absent,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Respawn {
    Resolved,
    Pending,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdmissionKind {
    NewLease,
    SameLeaseReconnect,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActorEnd {
    StillInWorld,
    RemovedBeforeTerminalRelease,
    AlreadyReleased,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlanError {
    InvalidLease,
    InvalidTime,
    InvalidColumns,
    MismatchedMarker,
    MismatchedActivation,
    InvalidGrant,
    InvalidBuild,
    UnknownVocation,
}

fn key(fence: &CurrentCharacterGameplayFence, kind: MarkerKind) -> Result<MarkerKey, PlanError> {
    if fence.character_lease_generation == 0 {
        return Err(PlanError::InvalidLease);
    }
    Ok(MarkerKey {
        character: fence.character_id,
        lease: fence.character_lease_generation,
        kind,
    })
}
fn existing(expected: MarkerKey, lookup: ProposedLookup) -> Result<bool, PlanError> {
    match lookup {
        ProposedLookup::Missing => Ok(false),
        ProposedLookup::Existing(value) if value == expected => Ok(true),
        _ => Err(PlanError::MismatchedMarker),
    }
}
fn times(
    fence: &CurrentCharacterGameplayFence,
    t: ProposedLeaseTimes,
    as_of_us: i64,
) -> Result<(), PlanError> {
    if t.acquired_at_us < 0
        || t.session_created_at_us < 0
        || as_of_us < t.acquired_at_us
        || as_of_us < t.session_created_at_us
    {
        return Err(PlanError::InvalidTime);
    }
    if t.predecessor.is_some_and(|p| {
        p.generation == 0
            || p.generation >= fence.character_lease_generation
            || p.session == fence.game_session_id
    }) {
        return Err(PlanError::InvalidLease);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkerCommitRequired {
    NativeRecoveryCurrentFencesClockCauseAndChain,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrainingRateOwner {
    TopVocationBaseAttackSpeedTimesTwo,
    TopVocationBaseAttackSpeedTimesFour,
    CurrentManaGainAndTopManaTicksWithOneSecondZeroFallback,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StaminaOwnerWork {
    pub(crate) before_minutes: u16,
    pub(crate) offline_s: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrainingWork {
    None,
    NeedsRates {
        skill: OfflineSkill,
        seconds: u32,
        build: DurableBuildState,
        owner: TrainingRateOwner,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettlementPlan {
    pub(crate) key: MarkerKey,
    pub(crate) proposed_as_of_us: i64,
    pub(crate) offline_s: u64,
    pub(crate) pool_after_s: u32,
    pub(crate) skill_after: Option<OfflineSkill>,
    pub(crate) initialize_native_progression_in_same_transaction: ProgressionRow,
    pub(crate) stamina: StaminaOwnerWork,
    pub(crate) training: TrainingWork,
    pub(crate) commit: MarkerCommitRequired,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SettlementDisposition {
    /// Requires real recovery validation and a native immutable receipt lookup/return. This
    /// carries no receipt/outcome or authority reconstructed from the raw proposed key.
    ReadExistingNativeMarker(MarkerKey),
    SameLeaseReconnect,
    WaitForRespawn,
    NeedsHouseScopeOwner,
    Plan(Box<SettlementPlan>),
}

#[allow(
    clippy::too_many_arguments,
    reason = "one concrete offline transition over native owner inputs"
)]
pub(crate) fn plan_settlement(
    fence: &CurrentCharacterGameplayFence,
    lease: ProposedLeaseTimes,
    as_of_us: i64,
    lookup: ProposedLookup,
    admission: AdmissionKind,
    respawn: Respawn,
    progression: ProgressionRow,
    latest: LatestBuildReceipt,
    before: ProposedOfflineColumns,
    build: &DurableBuildState,
) -> Result<SettlementDisposition, PlanError> {
    let key = key(fence, MarkerKind::Settlement)?;
    // Accepted marker replay is before binding/checks: a same-key retry returns the FIRST
    // native receipt. The production recovery/lookup is still a required owner integration.
    if existing(key, lookup)? {
        return Ok(SettlementDisposition::ReadExistingNativeMarker(key));
    }
    if admission == AdmissionKind::SameLeaseReconnect {
        return Ok(SettlementDisposition::SameLeaseReconnect);
    }
    if respawn == Respawn::Pending {
        return Ok(SettlementDisposition::WaitForRespawn);
    }
    if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. }) {
        return Ok(SettlementDisposition::NeedsHouseScopeOwner);
    }
    times(fence, lease, as_of_us)?;
    before.validate()?;
    let offline_s = match latest {
        LatestBuildReceipt::Logout(marker) => {
            if marker.key.kind != MarkerKind::Logout
                || marker.key.character != fence.character_id
                || marker.scope.world_id() != fence.runtime_scope.world_id()
            {
                return Err(PlanError::MismatchedMarker);
            }
            if marker.key.lease == 0 {
                return Err(PlanError::InvalidLease);
            }
            if marker.as_of_us < 0 || marker.as_of_us != marker.committed_at_us {
                return Err(PlanError::InvalidTime);
            }
            match lease.predecessor {
                Some(previous) if marker.key.lease == previous.generation => {
                    if marker.session != previous.session {
                        return Err(PlanError::MismatchedMarker);
                    }
                    u64::try_from(
                        lease
                            .acquired_at_us
                            .saturating_sub(marker.committed_at_us)
                            .max(0)
                            / SECOND_US,
                    )
                    .map_err(|_| PlanError::InvalidTime)?
                    .min(OFFLINE_CAP_S)
                }
                _ => 0,
            }
        }
        _ => 0,
    };
    let (pool_after_s, trained_s) = match before.skill {
        Some(_) if offline_s < 600 => (before.pool_s, 0),
        Some(_) => {
            let t = offline_s
                .min(u64::from(before.pool_s))
                .min(u64::from(POOL_CAP_S));
            let pool = (u64::from(before.pool_s) - t + (offline_s - t)).min(u64::from(POOL_CAP_S));
            (
                u32::try_from(pool).map_err(|_| PlanError::InvalidColumns)?,
                u32::try_from(t).map_err(|_| PlanError::InvalidTime)?,
            )
        }
        None => (
            u32::try_from((u64::from(before.pool_s) + offline_s).min(u64::from(POOL_CAP_S)))
                .map_err(|_| PlanError::InvalidColumns)?,
            0,
        ),
    };
    let training = match before.skill {
        Some(skill) if trained_s >= 60 => TrainingWork::NeedsRates {
            skill,
            seconds: trained_s,
            build: build.clone(),
            owner: match skill {
                OfflineSkill::MagicLevel => {
                    TrainingRateOwner::CurrentManaGainAndTopManaTicksWithOneSecondZeroFallback
                }
                OfflineSkill::Distance => TrainingRateOwner::TopVocationBaseAttackSpeedTimesFour,
                _ => TrainingRateOwner::TopVocationBaseAttackSpeedTimesTwo,
            },
        },
        _ => TrainingWork::None,
    };
    Ok(SettlementDisposition::Plan(Box::new(SettlementPlan {
        key,
        proposed_as_of_us: as_of_us,
        offline_s,
        pool_after_s,
        skill_after: None,
        initialize_native_progression_in_same_transaction: progression,
        stamina: StaminaOwnerWork {
            before_minutes: before.stamina_minutes,
            offline_s,
        },
        training,
        commit: MarkerCommitRequired::NativeRecoveryCurrentFencesClockCauseAndChain,
    })))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivationBlock {
    Clear,
    LogoutBlocked,
    PzLocked,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivationRefusal {
    Sealed,
    PzBlocked,
    InvalidContext,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivationOwnerStage {
    CurrentPremiumAndAdmittedStatueWorldUseReachPzCooldown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProposedActivation {
    /// Immutable expected binding only; the independently supplied logout fence must still
    /// match. This DTO is never a native current-authority witness.
    pub(crate) expected: CurrentCharacterGameplayFence,
    pub(crate) skill: OfflineSkill,
    pub(crate) still_required: ActivationOwnerStage,
}
/// A policy preview. Even Current produces only a PROPOSAL requiring the actual Premium and
/// WorldUse owners at execution, never a statue admission or graceful-logout authority token.
pub(crate) fn plan_statue_activation(
    fence: &CurrentCharacterGameplayFence,
    premium: &PremiumStatus<'_>,
    skill: OfflineSkill,
    block: ActivationBlock,
) -> Result<ProposedActivation, ActivationRefusal> {
    if fence.character_lease_generation == 0 {
        return Err(ActivationRefusal::InvalidContext);
    }
    premium
        .gate()
        .require()
        .map_err(|_| ActivationRefusal::Sealed)?;
    if block != ActivationBlock::Clear {
        return Err(ActivationRefusal::PzBlocked);
    }
    Ok(ProposedActivation {
        expected: *fence,
        skill,
        still_required:
            ActivationOwnerStage::CurrentPremiumAndAdmittedStatueWorldUseReachPzCooldown,
    })
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LogoutPlan {
    pub(crate) key: MarkerKey,
    pub(crate) proposed_as_of_us: i64,
    pub(crate) online_restore_s: u64,
    pub(crate) pool_after_s: u32,
    pub(crate) skill_after: Option<OfflineSkill>,
    pub(crate) activation: Option<ProposedActivation>,
    /// Final skills and stamina still come from the existing native runtime flush in this
    /// marker transaction, after actor removal and before terminal release. No copied engine.
    pub(crate) flush: FinalFlushOwnerStage,
    pub(crate) commit: MarkerCommitRequired,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FinalFlushOwnerStage {
    NativePendingTrainingAndStaminaAfterActorRemoval,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LogoutDisposition {
    ReadExistingNativeMarker(MarkerKey),
    WaitForActorRemoval,
    SkippedNoActivation,
    NeedsHouseScopeOwner,
    Plan(LogoutPlan),
}
#[allow(
    clippy::too_many_arguments,
    reason = "one concrete logout transition; no generic resolver"
)]
pub(crate) fn plan_logout(
    fence: &CurrentCharacterGameplayFence,
    lease: ProposedLeaseTimes,
    as_of_us: i64,
    lookup: ProposedLookup,
    settlement: ProposedLookup,
    end: ActorEnd,
    respawn: Respawn,
    progression: ProgressionRow,
    before: ProposedOfflineColumns,
    activation: Option<ProposedActivation>,
) -> Result<LogoutDisposition, PlanError> {
    let key = key(fence, MarkerKind::Logout)?;
    if existing(key, lookup)? {
        return Ok(LogoutDisposition::ReadExistingNativeMarker(key));
    }
    if end == ActorEnd::AlreadyReleased
        || respawn == Respawn::Pending
        || progression == ProgressionRow::Absent
    {
        return Ok(LogoutDisposition::SkippedNoActivation);
    }
    if end == ActorEnd::StillInWorld {
        return Ok(LogoutDisposition::WaitForActorRemoval);
    }
    if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. }) {
        return Ok(LogoutDisposition::NeedsHouseScopeOwner);
    }
    times(fence, lease, as_of_us)?;
    before.validate()?;
    let settled = existing(
        MarkerKey {
            kind: MarkerKind::Settlement,
            ..key
        },
        settlement,
    )?;
    let online_restore_s = if settled {
        u64::try_from((as_of_us - lease.session_created_at_us) / SECOND_US)
            .map_err(|_| PlanError::InvalidTime)?
    } else {
        0
    };
    let pool_after_s = u32::try_from(
        (u64::from(before.pool_s).saturating_add(online_restore_s)).min(u64::from(POOL_CAP_S)),
    )
    .map_err(|_| PlanError::InvalidColumns)?;
    if let Some(value) = activation
        && (value.expected.character_id != fence.character_id
            || value.expected.game_session_id != fence.game_session_id
            || value.expected.connection_generation != fence.connection_generation
            || value.expected.character_lease_generation != fence.character_lease_generation
            || value.expected.runtime_scope != fence.runtime_scope
            || value.expected.scope_ownership_generation != fence.scope_ownership_generation
            || before.skill.is_some_and(|skill| skill != value.skill))
    {
        return Err(PlanError::MismatchedActivation);
    }
    Ok(LogoutDisposition::Plan(LogoutPlan {
        key,
        proposed_as_of_us: as_of_us,
        online_restore_s,
        pool_after_s,
        skill_after: activation.map(|v| v.skill).or(before.skill),
        activation,
        flush: FinalFlushOwnerStage::NativePendingTrainingAndStaminaAfterActorRemoval,
        commit: MarkerCommitRequired::NativeRecoveryCurrentFencesClockCauseAndChain,
    }))
}

/// Separately sourced admitted delta is an owner obligation. This RAW number is not that proof:
/// previews below ALWAYS retain NeedsRates; they cannot complete or install a settlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RawTrainingDelta {
    Tries(u64),
    ManaSpent(u64),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PercentOwnerStage {
    NoChange(DurableBuildState),
    LevelChangedWithShielding(DurableBuildState),
    NeedsNativePercentProjection {
        without_shielding: DurableBuildState,
        if_percent_changed: DurableBuildState,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawTrainingPreview {
    pub(crate) rates_still_required: TrainingRateOwner,
    pub(crate) result: PercentOwnerStage,
}
pub(crate) fn preview_raw_training(
    work: &TrainingWork,
    delta: RawTrainingDelta,
    formula: &dyn BuildFormula,
) -> Result<RawTrainingPreview, PlanError> {
    let TrainingWork::NeedsRates {
        skill,
        seconds,
        build,
        owner,
    } = work
    else {
        return Err(PlanError::InvalidGrant);
    };
    if !(60..=POOL_CAP_S).contains(seconds) {
        return Err(PlanError::InvalidGrant);
    }
    if formula.required(build.vocation(), 0, 1).is_none() {
        return Err(PlanError::UnknownVocation);
    }
    let (index, amount) = match (*skill, delta) {
        (OfflineSkill::MagicLevel, RawTrainingDelta::ManaSpent(n)) => (None, n),
        (OfflineSkill::MagicLevel, _) => return Err(PlanError::InvalidGrant),
        (_, RawTrainingDelta::Tries(n)) => (
            Some(skill.native_index().ok_or(PlanError::InvalidGrant)?),
            n,
        ),
        _ => return Err(PlanError::InvalidGrant),
    };
    let mut skills = build.skills();
    let mut magic = build.magic();
    let before = index.map_or(magic, |i| skills[i]);
    // Reuse the actual native advancement loop and saturation. No curve/formula is copied.
    let after = relevel(before.1.saturating_add(amount), before.0, |next| {
        formula.required(build.vocation(), index.map_or(0, |i| i + 1), next)
    });
    if let Some(i) = index {
        skills[i] = after;
    } else {
        magic = after;
    }
    let primary = DurableBuildState::new(build.vocation(), magic, skills)
        .map_err(|_| PlanError::InvalidBuild)?;
    let result = if before == after {
        PercentOwnerStage::NoChange(primary)
    } else {
        let shield = SKILLS
            .iter()
            .position(|value| *value == "shielding")
            .ok_or(PlanError::InvalidBuild)?;
        let current = skills[shield];
        skills[shield] = relevel(
            current.1.saturating_add(u64::from(*seconds / 4)),
            current.0,
            |next| formula.required(build.vocation(), shield + 1, next),
        );
        let with = DurableBuildState::new(build.vocation(), magic, skills)
            .map_err(|_| PlanError::InvalidBuild)?;
        if after.0 != before.0 {
            PercentOwnerStage::LevelChangedWithShielding(with)
        } else {
            PercentOwnerStage::NeedsNativePercentProjection {
                without_shielding: primary,
                if_percent_changed: with,
            }
        }
    };
    Ok(RawTrainingPreview {
        rates_still_required: *owner,
        result,
    })
}

#[cfg(test)]
#[path = "offline_tests.rs"]
mod tests;

/// D1108: actual native marker writer. This source is integrated with the accompanying
/// character_build hunk and the explicitly named FND clock-source participant. It does
/// not supply admission/logout callers or statue/exercise behavior.
mod native_marker_writer {
    use super::*;
    use crate::durability::character_authority::{
        ReconciledCharacterAuthority, assert_recovery_fence,
    };
    use crate::durability::character_build::{
        BuildFormula, BuildOccurrence, NativeOfflineMarkerReceipt, NativeOfflineMarkerValues,
        load_offline_marker, lock_offline_build, write_offline_marker,
    };
    use crate::durability::character_progression::{
        CharacterProgressionError, ProgressionInitializationRequest,
        initialize_character_progression_in_transaction,
    };
    use crate::durability::character_vitals_stamina::regenerate_stamina_minutes;
    use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
    use crate::durability::{
        DurabilityError, DurabilityRoot, begin_semantic_transaction, commit_semantic_transaction,
        lock_admission_relations,
    };
    use sqlx::{Postgres, Transaction};
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    /// Source data is independent of marker receipts. Only the FND owner implements
    /// this port, by reading its current session/lease rows in the supplied transaction.
    /// It is not a pool getter, an authority grant, or a reconstruction from the marker.
    pub(crate) mod clock_source_sealed {
        pub(crate) trait Sealed {}
    }
    pub(crate) trait OfflineLeaseClockSource:
        clock_source_sealed::Sealed + Send + Sync
    {
        fn read_locked<'a>(
            &'a self,
            tx: &'a mut Transaction<'_, Postgres>,
            fence: &'a CurrentCharacterGameplayFence,
        ) -> Pin<Box<dyn Future<Output = Result<OfflineLeaseClockRead, DurabilityError>> + Send + 'a>>;
    }
    /// Ordinary typed SQL results, never a current-authority witness. The current
    /// gameplay fence is independently checked by the build owner in this same TX.
    pub(crate) struct OfflineLeaseClockRead {
        pub(crate) character: CharacterId,
        pub(crate) session: GameSessionId,
        pub(crate) world: crate::foundation::WorldId,
        pub(crate) lease_generation: u64,
        pub(crate) times: ProposedLeaseTimes,
    }

    /// Math participant of the existing build transaction, never a storage or
    /// clock owner. Its input is qualified by the native marker writer below.
    /// Both values are retained by that writer's one before/after receipt.
    struct SettlementStaminaParticipant {
        before_minutes: u16,
        after_minutes: u16,
    }

    impl SettlementStaminaParticipant {
        fn prepare(
            before_minutes: u16,
            qualified_offline_s: u64,
        ) -> Result<Self, OfflineMarkerError> {
            let after_minutes =
                regenerate_stamina_minutes(u32::from(before_minutes), qualified_offline_s)
                    .map_err(|_| OfflineMarkerError::InvalidState)?;
            Ok(Self {
                before_minutes,
                after_minutes: u16::try_from(after_minutes)
                    .map_err(|_| OfflineMarkerError::InvalidState)?,
            })
        }

        fn apply(self, columns: &mut ProposedOfflineColumns) -> Result<(), OfflineMarkerError> {
            if columns.stamina_minutes != self.before_minutes {
                return Err(OfflineMarkerError::InvalidState);
            }
            columns.stamina_minutes = self.after_minutes;
            Ok(())
        }
    }

    /// Immutable pinned Content facts only; not an authority witness. Implement
    /// alongside the actual native BuildFormula loader, never from actor input.
    pub(crate) mod training_source_sealed {
        pub(crate) trait Sealed {}
    }
    pub(crate) trait OfflineTrainingFactsSource:
        training_source_sealed::Sealed + Send + Sync
    {
        fn rate(&self, vocation: &str, skill: OfflineSkill) -> Option<OfflineTrainingRateFacts>;
        /// OFFLINE-0's changed-percent rule belongs to the qualified native
        /// projection, not an invented count inequality or guessed UI rounding.
        /// Called only when the selected family's level stayed equal.
        fn percent_changed(
            &self,
            formula: &dyn BuildFormula,
            vocation: &str,
            family: usize,
            before: (u16, u64),
            after: (u16, u64),
        ) -> Option<bool>;
    }
    pub(crate) struct OfflineTrainingRateFacts {
        pub(crate) vocation: String,
        pub(crate) top_vocation: String,
        pub(crate) content_revision: String,
        pub(crate) formula_digest: [u8; 32],
        pub(crate) rate: OfflineTrainingRate,
    }
    pub(crate) enum OfflineTrainingRate {
        Attack {
            top_base_attack_speed_ms: u32,
        },
        Magic {
            current_mana_gain: u32,
            top_mana_ticks_ms: u32,
        },
    }

    fn advance_offline_family(
        formula: &dyn BuildFormula,
        vocation: &str,
        family: usize,
        (level, stored): (u16, u64),
        gain: u64,
    ) -> Result<(u16, u64), OfflineMarkerError> {
        use crate::durability::character_build::{MAX_BUILD_LEVEL, MAX_BUILD_PROGRESS};
        // Existing native relevel accepts the current level as its floor, so
        // training adds to within-level progress without copying its advance loop.
        let after = relevel(
            stored.saturating_add(gain).min(MAX_BUILD_PROGRESS),
            level,
            |next| {
                formula
                    .required(vocation, family, next)
                    .filter(|required| *required > 0)
            },
        );
        if after.0 < MAX_BUILD_LEVEL && formula.required(vocation, family, after.0 + 1) == Some(0) {
            return Err(OfflineMarkerError::InvalidState);
        }
        Ok(after)
    }

    fn apply_offline_training(
        work: &TrainingWork,
        formula: &dyn BuildFormula,
        source: Option<&dyn OfflineTrainingFactsSource>,
        locked_build: &DurableBuildState,
    ) -> Result<DurableBuildState, OfflineMarkerError> {
        let TrainingWork::NeedsRates {
            skill,
            seconds,
            build,
            ..
        } = work
        else {
            return Ok(locked_build.clone());
        };
        if build != locked_build
            || *seconds < 60
            || *seconds > POOL_CAP_S
            || formula.required(build.vocation(), 0, 1).is_none()
        {
            return Err(OfflineMarkerError::InvalidState);
        }
        let source = source.ok_or(OfflineMarkerError::TrainingFactsRequired)?;
        let facts = source
            .rate(build.vocation(), *skill)
            .ok_or(OfflineMarkerError::TrainingFactsRequired)?;
        if facts.vocation != build.vocation()
            || facts.top_vocation.is_empty()
            || facts.top_vocation.len() > 128
            || facts.content_revision != formula.content_revision()
            || facts.formula_digest != formula.digest()
        {
            return Err(OfflineMarkerError::InvalidState);
        }
        let t = u128::from(*seconds);
        let gain = match (*skill, facts.rate) {
            (
                OfflineSkill::MagicLevel,
                OfflineTrainingRate::Magic {
                    current_mana_gain,
                    top_mana_ticks_ms,
                },
            ) => {
                let ticks = if top_mana_ticks_ms == 0 {
                    1000
                } else {
                    top_mana_ticks_ms
                };
                t * u128::from(current_mana_gain) * 1000 / u128::from(ticks)
            }
            (OfflineSkill::MagicLevel, _) | (_, OfflineTrainingRate::Magic { .. }) => {
                return Err(OfflineMarkerError::InvalidState);
            }
            (
                skill,
                OfflineTrainingRate::Attack {
                    top_base_attack_speed_ms,
                },
            ) => {
                if top_base_attack_speed_ms == 0 {
                    return Err(OfflineMarkerError::InvalidState);
                }
                let divisor = if skill == OfflineSkill::Distance {
                    4_u128
                } else {
                    2_u128
                };
                t * 1000 / (u128::from(top_base_attack_speed_ms) * divisor)
            }
        };
        let gain = u64::try_from(gain).map_err(|_| OfflineMarkerError::InvalidState)?;
        let family = match skill {
            OfflineSkill::MagicLevel => 0,
            OfflineSkill::Fist => 1,
            OfflineSkill::Club => 2,
            OfflineSkill::Sword => 3,
            OfflineSkill::Axe => 4,
            OfflineSkill::Distance => 5,
        };
        let mut magic = build.magic();
        let mut skills = build.skills();
        let before = if family == 0 {
            magic
        } else {
            skills[family - 1]
        };
        let after = advance_offline_family(formula, build.vocation(), family, before, gain)?;
        if family == 0 {
            magic = after;
        } else {
            skills[family - 1] = after;
        }
        let changed = if before.0 != after.0 {
            true
        } else if before == after {
            false
        } else {
            source
                .percent_changed(formula, build.vocation(), family, before, after)
                .ok_or(OfflineMarkerError::TrainingFactsRequired)?
        };
        if changed {
            skills[5] = advance_offline_family(
                formula,
                build.vocation(),
                6,
                skills[5],
                u64::from(*seconds / 4),
            )?;
        }
        DurableBuildState::new(build.vocation(), magic, skills)
            .map_err(OfflineMarkerError::Progression)
    }

    #[allow(
        clippy::large_enum_variant,
        reason = "one transient actor-end checkpoint retains both native build snapshots"
    )]
    #[derive(Debug, Clone)]
    pub(crate) enum OfflineMarkerCommand {
        Settlement {
            admission: AdmissionKind,
        },
        /// A2 supplies the final native flush after actor removal; the writer never
        /// derives it from a marker or discards live pending training itself.
        Logout {
            end: ActorEnd,
            before: DurableBuildState,
            after_flush: DurableBuildState,
            stamina_after_flush: u16,
        },
    }
    impl OfflineMarkerCommand {
        fn kind(&self) -> MarkerKind {
            match self {
                Self::Settlement { .. } => MarkerKind::Settlement,
                Self::Logout { .. } => MarkerKind::Logout,
            }
        }
    }
    #[derive(Debug)]
    pub(crate) enum OfflineMarkerError {
        Progression(CharacterProgressionError),
        Backend(DurabilityError),
        InvalidTimeOrClockBinding,
        InvalidState,
        /// No guessed top-vocation rate or percent rule is substituted. This refusal
        /// happens before any write; the pending activation remains durable for retry.
        TrainingFactsRequired,
    }
    impl From<DurabilityError> for OfflineMarkerError {
        fn from(value: DurabilityError) -> Self {
            Self::Backend(value)
        }
    }
    impl From<sqlx::Error> for OfflineMarkerError {
        fn from(value: sqlx::Error) -> Self {
            Self::Backend(value.into())
        }
    }
    #[derive(Debug)]
    pub(crate) enum OfflineMarkerOutcome {
        Committed(NativeOfflineMarkerReceipt),
        AlreadyCommitted(NativeOfflineMarkerReceipt),
        SameLeaseReconnect,
        WaitForRespawn,
        WaitForActorRemoval,
        Skipped,
    }

    impl DurabilityRoot {
        /// Called only by the matching Character RevisionSlot. The source port and
        /// formula are native owners; no completed result escapes before real COMMIT.
        #[allow(
            clippy::too_many_arguments,
            reason = "one fenced marker command with its independent clock/content participants"
        )]
        pub(crate) async fn commit_character_offline_marker<const N: usize>(
            &self,
            authority: &ReconciledCharacterAuthority<'_, '_>,
            node: &NodeIncarnationProof,
            fence: CurrentCharacterGameplayFence,
            occurrence: BuildOccurrence,
            command: OfflineMarkerCommand,
            clocks: Arc<dyn OfflineLeaseClockSource>,
            formula: Arc<dyn BuildFormula>,
            training_facts: Option<Arc<dyn OfflineTrainingFactsSource>>,
            initialization: Option<ProgressionInitializationRequest<N>>,
        ) -> Result<OfflineMarkerOutcome, OfflineMarkerError> {
            let marker_key =
                key(&fence, command.kind()).map_err(|_| OfflineMarkerError::InvalidState)?;
            let recovery = authority.record_for(self).map_err(|_| {
                OfflineMarkerError::Progression(CharacterProgressionError::AuthorityRejected)
            })?;
            let node = node.clone();
            self.try_issue_semantic_pass()?
                .run(move |holder, deadline| {
                    Box::pin(async move {
                        let mut tx = begin_semantic_transaction(holder, deadline).await?;
                        assert_recovery_fence(&mut tx, &recovery).await?;
                        lock_admission_relations(&mut tx).await?;
                        let outcome = apply_marker(
                            &mut tx,
                            &node,
                            fence,
                            marker_key,
                            occurrence,
                            command,
                            clocks.as_ref(),
                            formula.as_ref(),
                            training_facts.as_deref(),
                            initialization.as_ref(),
                        )
                        .await;
                        if outcome.is_ok() {
                            commit_semantic_transaction(tx, deadline).await?;
                        }
                        // Error drops the whole physical transaction, including D88 init.
                        Ok(outcome)
                    })
                })
                .await?
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn apply_marker<const N: usize>(
        tx: &mut Transaction<'_, Postgres>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        marker_key: MarkerKey,
        occurrence: BuildOccurrence,
        command: OfflineMarkerCommand,
        clocks: &dyn OfflineLeaseClockSource,
        formula: &dyn BuildFormula,
        training_facts: Option<&dyn OfflineTrainingFactsSource>,
        initialization: Option<&ProgressionInitializationRequest<N>>,
    ) -> Result<OfflineMarkerOutcome, OfflineMarkerError> {
        // The complete typed key decides equality. Hashing is a serialization aid only.
        sqlx::query(
            "SELECT pg_advisory_xact_lock(hashtextextended(\
            'oteryn:offline-marker:' || encode($1,'hex') || ':' || $2 || ':' || $3, 0))",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(fence.character_lease_generation.to_string())
        .bind(marker_key.kind.sql_key())
        .execute(&mut **tx)
        .await?;
        // OFFLINE-0 key replay precedes raw intent/clock/formula/state checks. The
        // historical receipt is returned only; it never becomes current authority.
        if let Some(receipt) = load_offline_marker(tx, marker_key).await? {
            return Ok(OfflineMarkerOutcome::AlreadyCommitted(receipt));
        }
        // Share the native build occurrence lock with ordinary build writers. A
        // reused occurrence under another marker key is a typed refusal, not a
        // late uniqueness error after a partial mutation.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-build:' || encode($1,'hex'), 0))")
            .bind(occurrence.as_bytes().as_slice()).execute(&mut **tx).await?;
        let reused: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_build_receipts WHERE build_occurrence_id=encode($1,'hex')::uuid)")
            .bind(occurrence.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        if reused {
            return Err(OfflineMarkerError::Progression(
                CharacterProgressionError::ConflictingOccurrence,
            ));
        }
        if matches!(
            command,
            OfflineMarkerCommand::Settlement {
                admission: AdmissionKind::SameLeaseReconnect
            }
        ) {
            return Ok(OfflineMarkerOutcome::SameLeaseReconnect);
        }
        if matches!(
            command,
            OfflineMarkerCommand::Logout {
                end: ActorEnd::AlreadyReleased,
                ..
            }
        ) {
            return Ok(OfflineMarkerOutcome::Skipped);
        }
        if matches!(
            command,
            OfflineMarkerCommand::Logout {
                end: ActorEnd::StillInWorld,
                ..
            }
        ) {
            return Ok(OfflineMarkerOutcome::WaitForActorRemoval);
        }
        // This native owner performs assert_gameplay_fence, locks the Character root,
        // then loads the real pending-respawn/build/progression rows in this TX.
        let locked = lock_offline_build(tx, node, &fence)
            .await
            .map_err(OfflineMarkerError::Progression)?;
        if locked.respawn_pending {
            return Ok(match marker_key.kind {
                MarkerKind::Settlement => OfflineMarkerOutcome::WaitForRespawn,
                MarkerKind::Logout => OfflineMarkerOutcome::Skipped,
            });
        }
        if !locked.progression_present {
            if marker_key.kind == MarkerKind::Logout {
                return Ok(OfflineMarkerOutcome::Skipped);
            }
            let init = initialization.ok_or(OfflineMarkerError::Progression(
                CharacterProgressionError::MissingProgressionState,
            ))?;
            initialize_character_progression_in_transaction(tx, node, fence, init)
                .await
                .map_err(OfflineMarkerError::Progression)?;
        }
        let clocks_read = clocks.read_locked(tx, &fence).await?;
        if clocks_read.character != fence.character_id
            || clocks_read.session != fence.game_session_id
            || clocks_read.world != fence.runtime_scope.world_id()
            || clocks_read.lease_generation != fence.character_lease_generation
        {
            return Err(OfflineMarkerError::InvalidTimeOrClockBinding);
        }
        // Read the database clock ONCE after the native root lock. No runtime/client
        // clock, receipt-derived lease anchor or unchecked ms -> us conversion exists.
        let as_of_us: i64 = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM clock_timestamp()) * 1000000)::bigint",
        )
        .fetch_one(&mut **tx)
        .await?;
        times(&fence, clocks_read.times, as_of_us)
            .map_err(|_| OfflineMarkerError::InvalidTimeOrClockBinding)?;
        let latest =
            crate::durability::character_build::latest_offline_build_receipt(tx, &fence).await?;
        let settled = load_offline_marker(
            tx,
            MarkerKey {
                kind: MarkerKind::Settlement,
                ..marker_key
            },
        )
        .await?
        .is_some();
        let (after_build, after_columns) = match command {
            OfflineMarkerCommand::Settlement { .. } => {
                let planned = plan_settlement(
                    &fence,
                    clocks_read.times,
                    as_of_us,
                    ProposedLookup::Missing,
                    AdmissionKind::NewLease,
                    Respawn::Resolved,
                    ProgressionRow::Present,
                    latest,
                    locked.columns,
                    &locked.build,
                )
                .map_err(|_| OfflineMarkerError::InvalidState)?;
                let SettlementDisposition::Plan(plan) = planned else {
                    return Err(OfflineMarkerError::InvalidState);
                };
                let after_build =
                    apply_offline_training(&plan.training, formula, training_facts, &locked.build)?;
                let stamina = SettlementStaminaParticipant::prepare(
                    locked.columns.stamina_minutes,
                    plan.offline_s,
                )?;
                let mut after_columns = ProposedOfflineColumns {
                    pool_s: plan.pool_after_s,
                    skill: None,
                    ..locked.columns
                };
                stamina.apply(&mut after_columns)?;
                (after_build, after_columns)
            }
            OfflineMarkerCommand::Logout {
                before,
                after_flush,
                stamina_after_flush,
                ..
            } => {
                if before != locked.build || stamina_after_flush > locked.columns.stamina_minutes {
                    return Err(OfflineMarkerError::Progression(
                        CharacterProgressionError::BuildStateMismatch,
                    ));
                }
                let planned = plan_logout(
                    &fence,
                    clocks_read.times,
                    as_of_us,
                    ProposedLookup::Missing,
                    if settled {
                        ProposedLookup::Existing(MarkerKey {
                            kind: MarkerKind::Settlement,
                            ..marker_key
                        })
                    } else {
                        ProposedLookup::Missing
                    },
                    ActorEnd::RemovedBeforeTerminalRelease,
                    Respawn::Resolved,
                    ProgressionRow::Present,
                    locked.columns,
                    None,
                )
                .map_err(|_| OfflineMarkerError::InvalidState)?;
                let LogoutDisposition::Plan(plan) = planned else {
                    return Err(OfflineMarkerError::InvalidState);
                };
                (
                    after_flush,
                    ProposedOfflineColumns {
                        stamina_minutes: stamina_after_flush,
                        pool_s: plan.pool_after_s,
                        skill: locked.columns.skill,
                    },
                )
            }
        };
        let receipt = write_offline_marker(
            tx,
            node,
            &fence,
            occurrence,
            formula,
            NativeOfflineMarkerValues {
                key: marker_key,
                as_of_us,
                before_build: locked.build,
                after_build,
                before_columns: locked.columns,
                after_columns,
            },
        )
        .await
        .map_err(OfflineMarkerError::Progression)?;
        Ok(OfflineMarkerOutcome::Committed(receipt))
    }
    #[cfg(test)]
    mod training_participant_tests {
        use super::*;

        struct Formula;
        impl BuildFormula for Formula {
            fn required(&self, _: &str, _: usize, _: u16) -> Option<u64> {
                Some(100)
            }
            fn digest(&self) -> [u8; 32] {
                [3; 32]
            }
            fn content_revision(&self) -> &str {
                "test-content"
            }
        }
        // Arithmetic-only fixture: not a runtime Content participant or authority.
        struct Facts {
            percent_changed: bool,
        }
        impl training_source_sealed::Sealed for Facts {}
        impl OfflineTrainingFactsSource for Facts {
            fn rate(
                &self,
                vocation: &str,
                skill: OfflineSkill,
            ) -> Option<OfflineTrainingRateFacts> {
                Some(OfflineTrainingRateFacts {
                    vocation: vocation.to_owned(),
                    top_vocation: "test-top".into(),
                    content_revision: Formula.content_revision().into(),
                    formula_digest: Formula.digest(),
                    rate: if skill == OfflineSkill::MagicLevel {
                        OfflineTrainingRate::Magic {
                            current_mana_gain: 3,
                            top_mana_ticks_ms: 0,
                        }
                    } else {
                        OfflineTrainingRate::Attack {
                            top_base_attack_speed_ms: 2000,
                        }
                    },
                })
            }
            fn percent_changed(
                &self,
                _: &dyn BuildFormula,
                _: &str,
                _: usize,
                _: (u16, u64),
                _: (u16, u64),
            ) -> Option<bool> {
                Some(self.percent_changed)
            }
        }
        fn work(skill: OfflineSkill, seconds: u32) -> TrainingWork {
            TrainingWork::NeedsRates {
                skill,
                seconds,
                build: DurableBuildState::default(),
                owner: TrainingRateOwner::TopVocationBaseAttackSpeedTimesTwo,
            }
        }
        #[test]
        fn selected_gain_advances_and_only_changed_level_or_percent_trains_shielding() {
            let before = DurableBuildState::default();
            let after = apply_offline_training(
                &work(OfflineSkill::Sword, 600),
                &Formula,
                Some(&Facts {
                    percent_changed: false,
                }),
                &before,
            )
            .unwrap_or_default();
            assert_eq!(after.skills()[2], (11, 50));
            assert_eq!(after.skills()[5], (11, 50));
            assert_eq!(after.skills()[0], before.skills()[0]);
            let unchanged_percent = apply_offline_training(
                &work(OfflineSkill::Distance, 60),
                &Formula,
                Some(&Facts {
                    percent_changed: false,
                }),
                &before,
            )
            .unwrap_or_default();
            assert_eq!(unchanged_percent.skills()[4], (10, 7));
            assert_eq!(unchanged_percent.skills()[5], (10, 0));
            let changed_percent = apply_offline_training(
                &work(OfflineSkill::Distance, 60),
                &Formula,
                Some(&Facts {
                    percent_changed: true,
                }),
                &before,
            )
            .unwrap_or_default();
            assert_eq!(changed_percent.skills()[5], (10, 15));
        }
        #[test]
        fn magic_zero_tick_fallback_is_one_second_and_missing_source_refuses() {
            let before = DurableBuildState::default();
            let after = apply_offline_training(
                &work(OfflineSkill::MagicLevel, 60),
                &Formula,
                Some(&Facts {
                    percent_changed: false,
                }),
                &before,
            )
            .unwrap_or_default();
            assert_eq!(after.magic(), (1, 80));
            assert_eq!(after.skills()[5], (10, 15));
            assert!(matches!(
                apply_offline_training(&work(OfflineSkill::Sword, 600), &Formula, None, &before),
                Err(OfflineMarkerError::TrainingFactsRequired)
            ));
        }
    }
}

impl MarkerKind {
    pub(crate) const fn sql_key(self) -> &'static str {
        match self {
            Self::Logout => "logout",
            Self::Settlement => "offline_settlement",
        }
    }
}
impl OfflineSkill {
    pub(crate) const fn sql_key(self) -> &'static str {
        match self {
            Self::Fist => "fist",
            Self::Club => "club",
            Self::Sword => "sword",
            Self::Axe => "axe",
            Self::Distance => "distance",
            Self::MagicLevel => "magic_level",
        }
    }
    pub(crate) fn from_sql_key(value: &str) -> Option<Self> {
        match value {
            "fist" => Some(Self::Fist),
            "club" => Some(Self::Club),
            "sword" => Some(Self::Sword),
            "axe" => Some(Self::Axe),
            "distance" => Some(Self::Distance),
            "magic_level" => Some(Self::MagicLevel),
            _ => None,
        }
    }
}

pub(crate) use native_marker_writer::clock_source_sealed as offline_clock_source_sealed;
#[allow(
    unused_imports,
    reason = "Pending CP/FND source integration retains the sealed training port."
)]
pub(crate) use native_marker_writer::training_source_sealed as offline_training_source_sealed;
#[allow(
    unused_imports,
    reason = "Pending CP source integration retains typed training rate ports."
)]
pub(crate) use native_marker_writer::{
    OfflineLeaseClockRead, OfflineLeaseClockSource, OfflineMarkerCommand, OfflineMarkerError,
    OfflineMarkerOutcome, OfflineTrainingFactsSource, OfflineTrainingRate,
    OfflineTrainingRateFacts,
};
