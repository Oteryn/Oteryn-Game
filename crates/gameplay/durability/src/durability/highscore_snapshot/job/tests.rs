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

#[test]
fn only_a_confirmed_success_ends_the_prompt_reset_retry() {
    use crate::durability::{MigrationFailureClass, SchemaCompatibility, SqlxFailureClass};
    use crate::foundation::admission_authority_publication::GameSessionUseAuthorizationErrorV1;

    let success = Ok(SnapshotRefresh {
        snapshots: Vec::new(),
        next_check_after: Duration::from_secs(30 * 60),
    });
    assert!(!retry_soon_after_pass(&success, true));
    assert!(!retry_soon_after_pass(&success, false));

    let errors = [
        SnapshotError::NotSupported,
        SnapshotError::InvalidQuery,
        SnapshotError::SnapshotExpired,
        SnapshotError::RetryableConflict,
        SnapshotError::CapacityExceeded,
    ];
    let durability_errors = [
        DurabilityError::AdmissionGameSessionLedgerExhausted,
        DurabilityError::SessionUseAuthorization(GameSessionUseAuthorizationErrorV1::StaleAuthority),
        DurabilityError::Unavailable,
        DurabilityError::Database(SqlxFailureClass::RetryableStatement),
        DurabilityError::Database(SqlxFailureClass::Transport),
        DurabilityError::Migration(MigrationFailureClass::Source),
        DurabilityError::SchemaIncompatible(SchemaCompatibility::Incompatible),
        DurabilityError::InvalidConfiguration,
        DurabilityError::RootUnavailable,
        DurabilityError::RootPassDeadlineExceeded,
        DurabilityError::CommitRejected,
        DurabilityError::CommitOutcomeUnknown,
        DurabilityError::RootTaskFailed,
        DurabilityError::InvalidStoredState,
    ];

    for error in errors
        .into_iter()
        .chain(durability_errors.into_iter().map(SnapshotError::Unavailable))
    {
        let ordinary_retry = contended(&error);
        let outcome = Err(error);
        assert!(retry_soon_after_pass(&outcome, true), "{outcome:?}");
        assert_eq!(retry_soon_after_pass(&outcome, false), ordinary_retry, "{outcome:?}");
    }
}
