//! Private owner bookkeeping for TRAVEL0 §7. No route admission, fee commit, actor
//! relocation or placement capability is created by this state machine. Its caller owns one
//! slot per actor, drops queued steps at `start`, refuses walking/logout/transfer while held,
//! and continues combat. Only the independently current native owner may apply a completion.

#![allow(
    dead_code,
    reason = "Private accepted travel component; production owner integration remains a CP slot"
)]

use std::time::{Duration, Instant};

use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::npc_travel::{ConsumeReplay, PendingArrival, TravelOccurrence};
use crate::foundation::{CommandRef, RuntimeScopeRefV1, RuntimeWorkStamp, ScopeRuntimeFence};
use crate::world_runtime::ReferenceContentGeneration;

const AMBIGUITY_ALERT_AFTER: Duration = Duration::from_millis(2_000);

/// Immutable confirmed server-side facts. Construction does NOT admit a route or prove
/// eligibility; the actual talk/content/quest/Premium/fee source must supply those checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenTravel {
    pub command: CommandRef,
    pub npc: String,
    pub route: String,
    pub fare: u64,
    pub arrival: PendingArrival,
}

/// A capture to compare with independent current owner/session/content/position facts.
/// These values are not themselves current authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TravelCapture {
    pub session: CurrentCharacterItemFence,
    pub content: ReferenceContentGeneration,
    pub position_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeeOutcome {
    Committed,
    Aborted,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    FeePending,
    PlacementPending,
    ConsumePending,
    Completed,
    Aborted,
    CancelledByDeath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TravelStep {
    Wait,
    PlacementPending,
    RetryConsume,
    Complete,
    LeaveForAdmission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldError {
    Busy,
    InvalidBinding,
    StaleCompletion,
    InvalidPhase,
    NonMonotonicTime,
}

#[derive(Debug)]
struct Invocation {
    frozen: FrozenTravel,
    captured: TravelCapture,
    work: RuntimeWorkStamp,
    phase: Phase,
    phase_started: Instant,
}

/// At most one unresolved invocation per actor; a replay cannot replace its frozen binding.
#[derive(Debug, Default)]
pub struct TravelHoldSlot {
    invocation: Option<Invocation>,
}

impl TravelHoldSlot {
    pub fn start(
        &mut self,
        frozen: FrozenTravel,
        captured: TravelCapture,
        work: RuntimeWorkStamp,
        owner: &ScopeRuntimeFence,
        now: Instant,
    ) -> Result<(), HoldError> {
        if self.is_held() {
            return Err(HoldError::Busy);
        }
        if self
            .invocation
            .as_ref()
            .is_some_and(|i| i.frozen.arrival.occurrence == frozen.arrival.occurrence)
        {
            return Err(HoldError::StaleCompletion);
        }
        if frozen.npc.is_empty()
            || frozen.route.is_empty()
            || frozen.command.game_session_id() != captured.session.game_session_id
            || !matches!(captured.session.runtime_scope,
                RuntimeScopeRefV1::Channel { world_id, .. } if world_id == frozen.arrival.world_id)
            || captured.session.character_lease_generation == 0
            || !owner.is_current_for_scope(
                captured.session.runtime_scope,
                captured.session.scope_ownership_generation,
            )
            || !owner.accepts_stamp(work)
        {
            return Err(HoldError::InvalidBinding);
        }
        self.invocation = Some(Invocation {
            frozen,
            captured,
            work,
            phase: Phase::FeePending,
            phase_started: now,
        });
        Ok(())
    }

    pub fn is_held(&self) -> bool {
        self.invocation.as_ref().is_some_and(|i| {
            matches!(
                i.phase,
                Phase::FeePending | Phase::PlacementPending | Phase::ConsumePending
            )
        })
    }

    pub fn frozen(&self) -> Option<&FrozenTravel> {
        self.invocation.as_ref().map(|i| &i.frozen)
    }

    /// Alert only: elapsed time NEVER releases the hold or classifies a database outcome.
    pub fn ambiguity_alert_due(&self, now: Instant) -> Result<bool, HoldError> {
        let Some(i) = &self.invocation else {
            return Ok(false);
        };
        let elapsed = now
            .checked_duration_since(i.phase_started)
            .ok_or(HoldError::NonMonotonicTime)?;
        Ok(matches!(i.phase, Phase::FeePending | Phase::ConsumePending)
            && elapsed >= AMBIGUITY_ALERT_AFTER)
    }

    /// Feed only the actual fee/cause transaction's known or replayed disposition. Presence
    /// or absence of a pending-arrival row alone cannot classify this initial transaction.
    pub fn fee_outcome(
        &mut self,
        occurrence: TravelOccurrence,
        outcome: FeeOutcome,
    ) -> Result<TravelStep, HoldError> {
        let i = self.matching(occurrence)?;
        if i.phase == Phase::CancelledByDeath {
            return Ok(TravelStep::Complete);
        }
        if i.phase != Phase::FeePending {
            return Ok(TravelStep::Wait);
        }
        match outcome {
            FeeOutcome::Ambiguous => Ok(TravelStep::Wait),
            FeeOutcome::Aborted => {
                i.phase = Phase::Aborted;
                Ok(TravelStep::Complete)
            }
            FeeOutcome::Committed => {
                i.phase = Phase::PlacementPending;
                Ok(TravelStep::PlacementPending)
            }
        }
    }

    /// A bookkeeping decision, NEVER an executable relocation grant. The caller must also
    /// hold the actual actor owner/collision/relocation capability and apply all §7.3 checks.
    pub fn placement_decision(
        &self,
        current: &TravelCapture,
        owner: &ScopeRuntimeFence,
        alive: bool,
        death_in_progress: bool,
    ) -> TravelStep {
        let Some(i) = &self.invocation else {
            return TravelStep::Complete;
        };
        if i.phase != Phase::PlacementPending {
            return TravelStep::Wait;
        }
        if !alive
            || death_in_progress
            || &i.captured != current
            || !owner.is_current_for_scope(
                i.captured.session.runtime_scope,
                i.captured.session.scope_ownership_generation,
            )
            || !owner.accepts_stamp(i.work)
        {
            return TravelStep::LeaveForAdmission;
        }
        TravelStep::PlacementPending
    }

    /// Called only AFTER actual owner-fenced placement/capture, never merely because the
    /// fee committed. The follow-up consume uses that actual native FinalLastPosition.
    pub fn placement_recorded(
        &mut self,
        occurrence: TravelOccurrence,
        now: Instant,
    ) -> Result<(), HoldError> {
        let i = self.matching(occurrence)?;
        if i.phase != Phase::PlacementPending {
            return Err(HoldError::InvalidPhase);
        }
        if now < i.phase_started {
            return Err(HoldError::NonMonotonicTime);
        }
        i.phase = Phase::ConsumePending;
        i.phase_started = now;
        Ok(())
    }

    /// A known consume/no-op, or its session-fenced row reconciliation. Unknown outcomes
    /// do not call this method and retain the hold. Retry never requests another relocation.
    pub fn consume_outcome(
        &mut self,
        occurrence: TravelOccurrence,
        replay: ConsumeReplay,
    ) -> Result<TravelStep, HoldError> {
        let i = self.matching(occurrence)?;
        if i.phase == Phase::CancelledByDeath {
            return Ok(TravelStep::Complete);
        }
        if i.phase != Phase::ConsumePending {
            return Err(HoldError::InvalidPhase);
        }
        match replay {
            ConsumeReplay::RetryConsume => Ok(TravelStep::RetryConsume),
            ConsumeReplay::Complete => {
                i.phase = Phase::Completed;
                Ok(TravelStep::Complete)
            }
        }
    }

    /// Owner death occurrence, BEFORE the death DB transaction. Cancels even an ambiguous
    /// fee/consume, releases the hold, and makes every late completion inert. The actual
    /// death transaction must delete the arrival and receipt its occurrence; no refund.
    pub fn cancel_for_death(&mut self) {
        if let Some(i) = &mut self.invocation {
            i.phase = Phase::CancelledByDeath;
        }
    }

    fn matching(&mut self, occurrence: TravelOccurrence) -> Result<&mut Invocation, HoldError> {
        self.invocation
            .as_mut()
            .filter(|i| i.frozen.arrival.occurrence == occurrence)
            .ok_or(HoldError::StaleCompletion)
    }
}

#[cfg(test)]
#[path = "npc_travel_tests.rs"]
mod tests;
