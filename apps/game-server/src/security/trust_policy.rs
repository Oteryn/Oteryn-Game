//! Numeric SEC_CLIENT01 §3 constraints, independent of transport and signed schema.
//!
//! Durations share an epoch/unit supplied by the future owner; no wire timestamp
//! format is selected here. Inputs are facts to check, not authenticated evidence.
//! Signature/root lineage, serialized storage, embedded seed provenance and fresh
//! retrieval remain obligations of the unwired trust owner. No result is VERIFIED.

use std::time::Duration;

pub(crate) const MAX_CHECKPOINT_AGE: Duration = Duration::from_secs(24 * 60 * 60);
pub(crate) const MAX_CLOCK_SKEW: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PolicyError {
    AtOrBelowFloor,
    WrongPredecessor,
    RevisionExhausted,
    NonconsecutiveRevision,
    CheckpointBelowFloor,
    CheckpointRegressed,
    CheckpointBeforeEmbedded,
    CheckpointExpired,
    CheckpointTooFarFuture,
    AppliedHeadMismatch,
    MonotonicRegressed,
    ClockOverflow,
    FetchNotAfterStep,
}

/// Ordinary record advance only. A DB below the embedded floor cannot bootstrap
/// by applying the record at that floor; seed/recovery is a separate contract.
pub(crate) fn check_record_advance(
    applied: u64,
    embedded_floor: u64,
    previous_revision: u64,
    revision: u64,
) -> Result<(), PolicyError> {
    let next = applied
        .checked_add(1)
        .ok_or(PolicyError::RevisionExhausted)?;
    if revision <= applied.max(embedded_floor) {
        return Err(PolicyError::AtOrBelowFloor);
    }
    if previous_revision != applied {
        return Err(PolicyError::WrongPredecessor);
    }
    if revision != next {
        return Err(PolicyError::NonconsecutiveRevision);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CheckpointMetadata {
    pub revision: u64,
    pub issued_at: Duration,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CheckpointConstraints {
    pub embedded_floor: u64,
    pub highest_seen_revision: u64,
    pub embedded_issued_at: Duration,
    pub now: Duration,
}

/// Checks only numeric metadata; an unsigned checkpoint can pass this function.
/// The caller must separately authenticate the pinned root and checkpoint bytes.
pub(crate) fn check_checkpoint_metadata(
    checkpoint: CheckpointMetadata,
    constraints: CheckpointConstraints,
) -> Result<(), PolicyError> {
    if checkpoint.revision < constraints.embedded_floor {
        return Err(PolicyError::CheckpointBelowFloor);
    }
    if checkpoint.revision < constraints.highest_seen_revision {
        return Err(PolicyError::CheckpointRegressed);
    }
    if checkpoint.issued_at < constraints.embedded_issued_at {
        return Err(PolicyError::CheckpointBeforeEmbedded);
    }
    if let Some(age) = constraints.now.checked_sub(checkpoint.issued_at) {
        if age > MAX_CHECKPOINT_AGE {
            return Err(PolicyError::CheckpointExpired);
        }
    } else if checkpoint.issued_at - constraints.now > MAX_CLOCK_SKEW {
        return Err(PolicyError::CheckpointTooFarFuture);
    }
    Ok(())
}

/// Equal heads are necessary, never sufficient for usable challenge evidence.
pub(crate) fn check_applied_head(
    applied: u64,
    checkpoint_revision: u64,
) -> Result<(), PolicyError> {
    if applied != checkpoint_revision {
        return Err(PolicyError::AppliedHeadMismatch);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ClockSample {
    pub wall: Duration,
    pub monotonic: Duration,
}

/// Compare raw wall observations against elapsed monotonic time. Do not supply
/// World clock facts: their clamping deliberately hides wall-clock adjustments.
/// An error also fails closed; the future owner must latch either case.
pub(crate) fn backward_step_exceeds_skew(
    previous: ClockSample,
    current: ClockSample,
) -> Result<bool, PolicyError> {
    let elapsed = current
        .monotonic
        .checked_sub(previous.monotonic)
        .ok_or(PolicyError::MonotonicRegressed)?;
    let expected_wall = previous
        .wall
        .checked_add(elapsed)
        .ok_or(PolicyError::ClockOverflow)?;
    Ok(expected_wall
        .checked_sub(current.wall)
        .is_some_and(|behind| behind > MAX_CLOCK_SKEW))
}

/// Compare observations from the same process monotonic origin. This predicate
/// does not prove a fetch occurred or authenticate its result. The future owner
/// must retain its latch and fetch provenance, and recheck all checkpoint rules.
pub(crate) fn check_fetch_after_step(
    fetched_at: Duration,
    step_at: Duration,
) -> Result<(), PolicyError> {
    if fetched_at <= step_at {
        return Err(PolicyError::FetchNotAfterStep);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
