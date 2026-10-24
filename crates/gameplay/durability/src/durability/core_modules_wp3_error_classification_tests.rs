use super::{DurabilityError, SqlxFailureClass};
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Default)]
struct Observations {
    drops: Arc<AtomicUsize>,
    formats: Arc<AtomicUsize>,
}

struct PayloadProbe(Observations);

impl Drop for PayloadProbe {
    fn drop(&mut self) {
        self.0.drops.fetch_add(1, Ordering::SeqCst);
    }
}

impl fmt::Display for PayloadProbe {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.formats.fetch_add(1, Ordering::SeqCst);
        formatter.write_str("WP3_TEST_UNTRUSTED_PAYLOAD")
    }
}

impl fmt::Debug for PayloadProbe {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl std::error::Error for PayloadProbe {}

impl sqlx::error::DatabaseError for PayloadProbe {
    fn message(&self) -> &str {
        "redacted-test-database-error"
    }

    fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        self
    }

    fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
        self
    }

    fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
        self
    }

    fn kind(&self) -> sqlx::error::ErrorKind {
        sqlx::error::ErrorKind::Other
    }
}

#[test]
fn sqlx_payload_is_classified_then_destroyed_without_formatting() {
    let seen = Observations::default();
    let error = DurabilityError::from(sqlx::Error::Tls(Box::new(PayloadProbe(seen.clone()))));

    assert!(matches!(
        error,
        DurabilityError::Database(SqlxFailureClass::Tls)
    ));
    assert_eq!(seen.drops.load(Ordering::SeqCst), 1);
    assert_eq!(seen.formats.load(Ordering::SeqCst), 0);
    assert!(!format!("{error}").contains("WP3_TEST_UNTRUSTED_PAYLOAD"));
    assert!(!format!("{error:?}").contains("WP3_TEST_UNTRUSTED_PAYLOAD"));
    assert_eq!(seen.formats.load(Ordering::SeqCst), 0);
}

#[test]
fn database_commit_rejection_is_definitive_without_payload_retention() {
    let seen = Observations::default();
    let error = DurabilityError::from_commit_error(sqlx::Error::Database(Box::new(PayloadProbe(
        seen.clone(),
    ))));

    assert!(matches!(error, DurabilityError::CommitRejected));
    assert_eq!(seen.drops.load(Ordering::SeqCst), 1);
    assert_eq!(seen.formats.load(Ordering::SeqCst), 0);
}

#[test]
fn non_database_commit_error_stays_ambiguous_without_payload_retention() {
    let seen = Observations::default();
    let error =
        DurabilityError::from_commit_error(sqlx::Error::Tls(Box::new(PayloadProbe(seen.clone()))));

    assert!(matches!(error, DurabilityError::CommitOutcomeUnknown));
    assert_eq!(seen.drops.load(Ordering::SeqCst), 1);
    assert_eq!(seen.formats.load(Ordering::SeqCst), 0);
}
