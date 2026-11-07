//! Private owner bookkeeping for TRAVEL0 §7. No route admission, fee commit, actor
//! relocation or placement capability is created by this state machine. Its caller owns one
//! slot per actor, drops queued steps at `start`, refuses walking/logout/transfer while held,
//! and continues combat. Only the independently current native owner may apply a completion.

use crate::foundation::{RuntimeWorkStamp, ScopeRuntimeFence};
use std::time::{Duration, Instant};

const AMBIGUITY_ALERT_AFTER: Duration = Duration::from_millis(2_000);

/// Caller-owned immutable bindings and current authority checks. These methods do not
/// commit fees, move actors or classify unknown database outcomes.
pub trait TravelBinding: std::fmt::Debug {
    type Capture: PartialEq + std::fmt::Debug;
    type Occurrence: PartialEq;
    type Replay;

    fn occurrence(&self) -> Self::Occurrence;
    fn valid_start(
        &self,
        captured: &Self::Capture,
        work: RuntimeWorkStamp,
        owner: &ScopeRuntimeFence,
    ) -> bool;
    fn valid_placement(
        &self,
        captured: &Self::Capture,
        owner: &ScopeRuntimeFence,
        work: RuntimeWorkStamp,
    ) -> bool;
    fn retry_consume(replay: Self::Replay) -> bool;
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
struct Invocation<F: TravelBinding> {
    frozen: F,
    captured: F::Capture,
    work: RuntimeWorkStamp,
    phase: Phase,
    phase_started: Instant,
}

/// At most one unresolved invocation per actor; a replay cannot replace its frozen binding.
#[derive(Debug)]
pub struct TravelHoldSlot<F: TravelBinding> {
    invocation: Option<Invocation<F>>,
}

impl<F: TravelBinding> TravelHoldSlot<F> {
    pub fn start(
        &mut self,
        frozen: F,
        captured: F::Capture,
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
            .is_some_and(|i| i.frozen.occurrence() == frozen.occurrence())
        {
            return Err(HoldError::StaleCompletion);
        }
        if !frozen.valid_start(&captured, work, owner) {
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

    pub fn frozen(&self) -> Option<&F> {
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
        occurrence: F::Occurrence,
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
        current: &F::Capture,
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
            || !i.frozen.valid_placement(&i.captured, owner, i.work)
        {
            return TravelStep::LeaveForAdmission;
        }
        TravelStep::PlacementPending
    }

    /// Called only AFTER actual owner-fenced placement/capture, never merely because the
    /// fee committed. The follow-up consume uses that actual native FinalLastPosition.
    pub fn placement_recorded(
        &mut self,
        occurrence: F::Occurrence,
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
        occurrence: F::Occurrence,
        replay: F::Replay,
    ) -> Result<TravelStep, HoldError> {
        let i = self.matching(occurrence)?;
        if i.phase == Phase::CancelledByDeath {
            return Ok(TravelStep::Complete);
        }
        if i.phase != Phase::ConsumePending {
            return Err(HoldError::InvalidPhase);
        }
        if F::retry_consume(replay) {
            Ok(TravelStep::RetryConsume)
        } else {
            i.phase = Phase::Completed;
            Ok(TravelStep::Complete)
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

    fn matching(&mut self, occurrence: F::Occurrence) -> Result<&mut Invocation<F>, HoldError> {
        self.invocation
            .as_mut()
            .filter(|i| i.frozen.occurrence() == occurrence)
            .ok_or(HoldError::StaleCompletion)
    }
}

impl<F: TravelBinding> Default for TravelHoldSlot<F> {
    fn default() -> Self {
        Self { invocation: None }
    }
}
