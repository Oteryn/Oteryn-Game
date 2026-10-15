//! OFFLINE0 §§3–6: noncommitting marker/lease/pool/activation transition plans.
//!
//! Raw proposed marker/time/context data is not current authority or a native receipt. The
//! native DB-clock, recovery/session/scope/node, receipt-chain and marker-cause integration is
//! absent. Every fresh plan exposes required owner stages; no installable settlement or writer
//! is produced. Stamina belongs to its existing owner; native build arithmetic is reused.
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
