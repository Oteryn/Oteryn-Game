use super::*;

#[test]
fn a_contended_holder_retries_soon_and_other_failures_back_off_an_interval() {
    let interval = Duration::from_secs(30 * 60);
    let now = Instant::now();
    // A periodic pass runs at its deadline.
    let reached = now;
    assert_eq!(
        next_due_after_failure(true, false, now, reached, interval),
        now + CONTENDED_RETRY
    );
    assert_eq!(
        next_due_after_failure(false, false, now, reached, interval),
        now + interval
    );
    // A failed reset keeps the pending periodic deadline unless a contended retry is sooner.
    let pending = now + Duration::from_secs(60);
    assert_eq!(
        next_due_after_failure(false, true, now, pending, interval),
        pending
    );
    assert_eq!(
        next_due_after_failure(true, true, now, pending, interval),
        now + CONTENDED_RETRY
    );
    // A retried reset runs at a passed deadline; its failure never spins.
    assert_eq!(
        next_due_after_failure(false, true, now, reached, interval),
        now + interval
    );
    assert_eq!(
        next_due_after_failure(true, true, now, reached, interval),
        now + CONTENDED_RETRY
    );
}

#[test]
fn only_a_busy_holder_counts_as_contention() {
    assert!(contended(&SnapshotError::Unavailable(
        DurabilityError::RootUnavailable
    )));
    assert!(!contended(&SnapshotError::Unavailable(
        DurabilityError::Unavailable
    )));
    assert!(!contended(&SnapshotError::RetryableConflict));
    assert!(!contended(&SnapshotError::CapacityExceeded));
}
