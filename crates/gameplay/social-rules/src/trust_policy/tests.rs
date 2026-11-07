use super::*;

fn seconds(value: u64) -> Duration {
    Duration::from_secs(value)
}
fn constraints() -> CheckpointConstraints {
    CheckpointConstraints {
        embedded_floor: 7,
        highest_seen_revision: 8,
        embedded_issued_at: seconds(100),
        now: seconds(100_000),
    }
}
fn checkpoint() -> CheckpointMetadata {
    CheckpointMetadata {
        revision: 8,
        issued_at: seconds(90_000),
    }
}

#[test]
fn record_advance_is_consecutive_and_binds_predecessor() {
    assert_eq!(check_record_advance(8, 7, 8, 9), Ok(()));
    assert_eq!(
        check_record_advance(8, 7, 7, 9),
        Err(PolicyError::WrongPredecessor)
    );
    assert_eq!(
        check_record_advance(8, 7, 8, 10),
        Err(PolicyError::NonconsecutiveRevision)
    );
}

#[test]
fn ordinary_advance_cannot_cross_a_restored_embedded_floor() {
    assert_eq!(
        check_record_advance(8, 7, 8, 8),
        Err(PolicyError::AtOrBelowFloor)
    );
    assert_eq!(
        check_record_advance(6, 7, 6, 7),
        Err(PolicyError::AtOrBelowFloor)
    );
    assert_eq!(
        check_record_advance(6, 7, 6, 8),
        Err(PolicyError::NonconsecutiveRevision)
    );
}

#[test]
fn maximum_revision_never_wraps() {
    assert_eq!(
        check_record_advance(u64::MAX, 7, u64::MAX, 0),
        Err(PolicyError::RevisionExhausted)
    );
    assert_eq!(
        check_record_advance(u64::MAX - 1, 7, u64::MAX - 1, u64::MAX),
        Ok(())
    );
}

#[test]
fn checkpoint_revision_and_embedded_time_constraints_are_independent() {
    let facts = constraints();
    assert_eq!(check_checkpoint_metadata(checkpoint(), facts), Ok(()));
    assert_eq!(
        check_checkpoint_metadata(
            CheckpointMetadata {
                revision: 6,
                ..checkpoint()
            },
            CheckpointConstraints {
                highest_seen_revision: 6,
                ..facts
            }
        ),
        Err(PolicyError::CheckpointBelowFloor)
    );
    assert_eq!(
        check_checkpoint_metadata(
            CheckpointMetadata {
                revision: 7,
                ..checkpoint()
            },
            facts
        ),
        Err(PolicyError::CheckpointRegressed)
    );
    assert_eq!(
        check_checkpoint_metadata(
            CheckpointMetadata {
                issued_at: seconds(99),
                ..checkpoint()
            },
            facts
        ),
        Err(PolicyError::CheckpointBeforeEmbedded)
    );
}

#[test]
fn checkpoint_maximum_age_is_inclusive_and_expires_between_polls() {
    let candidate = checkpoint();
    let edge = candidate.issued_at + MAX_CHECKPOINT_AGE;
    assert_eq!(
        check_checkpoint_metadata(
            candidate,
            CheckpointConstraints {
                now: edge,
                ..constraints()
            }
        ),
        Ok(())
    );
    assert_eq!(
        check_checkpoint_metadata(
            candidate,
            CheckpointConstraints {
                now: edge + Duration::from_nanos(1),
                ..constraints()
            }
        ),
        Err(PolicyError::CheckpointExpired)
    );
}

#[test]
fn checkpoint_future_skew_is_inclusive() {
    let facts = constraints();
    let edge = facts.now + MAX_CLOCK_SKEW;
    assert_eq!(
        check_checkpoint_metadata(
            CheckpointMetadata {
                issued_at: edge,
                ..checkpoint()
            },
            facts
        ),
        Ok(())
    );
    assert_eq!(
        check_checkpoint_metadata(
            CheckpointMetadata {
                issued_at: edge + Duration::from_nanos(1),
                ..checkpoint()
            },
            facts
        ),
        Err(PolicyError::CheckpointTooFarFuture)
    );
}

#[test]
fn checkpoint_comparisons_do_not_overflow_near_maximum_time() {
    assert_eq!(
        check_checkpoint_metadata(
            CheckpointMetadata {
                issued_at: Duration::MAX,
                ..checkpoint()
            },
            CheckpointConstraints {
                now: Duration::MAX,
                ..constraints()
            }
        ),
        Ok(())
    );
}

#[test]
fn both_behind_and_ahead_applied_heads_refuse() {
    assert_eq!(check_applied_head(8, 8), Ok(()));
    assert_eq!(
        check_applied_head(7, 8),
        Err(PolicyError::AppliedHeadMismatch)
    );
    assert_eq!(
        check_applied_head(9, 8),
        Err(PolicyError::AppliedHeadMismatch)
    );
}

#[test]
fn backward_discrepancy_is_measured_against_monotonic_elapsed() {
    let previous = ClockSample {
        wall: seconds(1_000),
        monotonic: seconds(10),
    };
    let exact = ClockSample {
        wall: seconds(710),
        monotonic: seconds(20),
    };
    assert_eq!(backward_step_exceeds_skew(previous, exact), Ok(false));
    assert_eq!(
        backward_step_exceeds_skew(
            previous,
            ClockSample {
                wall: exact.wall - Duration::from_nanos(1),
                ..exact
            }
        ),
        Ok(true)
    );
    assert_eq!(
        backward_step_exceeds_skew(
            previous,
            ClockSample {
                wall: seconds(1_100),
                ..exact
            }
        ),
        Ok(false)
    );
}

#[test]
fn unusable_clock_observations_fail_closed() {
    let previous = ClockSample {
        wall: seconds(1_000),
        monotonic: seconds(10),
    };
    assert_eq!(
        backward_step_exceeds_skew(
            previous,
            ClockSample {
                wall: seconds(1_001),
                monotonic: seconds(9)
            }
        ),
        Err(PolicyError::MonotonicRegressed)
    );
    assert_eq!(
        backward_step_exceeds_skew(
            ClockSample {
                wall: Duration::MAX,
                ..previous
            },
            ClockSample {
                wall: Duration::MAX,
                monotonic: seconds(11)
            }
        ),
        Err(PolicyError::ClockOverflow)
    );
}

#[test]
fn pre_step_and_equal_time_fetches_cannot_clear_a_latch() {
    assert_eq!(
        check_fetch_after_step(seconds(9), seconds(10)),
        Err(PolicyError::FetchNotAfterStep)
    );
    assert_eq!(
        check_fetch_after_step(seconds(10), seconds(10)),
        Err(PolicyError::FetchNotAfterStep)
    );
    assert_eq!(check_fetch_after_step(seconds(11), seconds(10)), Ok(()));
}
