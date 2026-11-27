//! Snapshot diagnostics and the bounded wait for the shared durability holder.
use crate::durability::{DurabilityError, DurabilityRoot};
use oteryn_error_codes::{FieldValue, Level, Line, emit};
use std::future::Future;
use std::hash::{BuildHasher, RandomState};
use std::sync::LazyLock;
use std::time::Duration;
use tokio::time::Instant;

const CHECKOUT_BUDGET: Duration = Duration::from_secs(1);
const HOLDER_POLL: Duration = Duration::from_millis(50);

pub(super) struct SnapshotTrace {
    account_ref: u64,
    received: Instant,
    awaiting_result: bool,
    pub(super) revision: Option<u64>,
}

impl SnapshotTrace {
    pub(super) fn new(account: [u8; 16]) -> Self {
        // Correlate the same account across admissions on this process without
        // publishing its AccountId or carrying a join key across restarts.
        static ACCOUNT_REFS: LazyLock<RandomState> = LazyLock::new(RandomState::new);
        Self {
            account_ref: ACCOUNT_REFS.hash_one(account),
            received: Instant::now(),
            awaiting_result: false,
            revision: None,
        }
    }

    fn fields<'a>(&self, line: Line<'a>, reason: &'a str) -> Line<'a> {
        let revision = self
            .revision
            .map_or(FieldValue::Token("unknown"), FieldValue::U64);
        line.field(
            "account_ref",
            FieldValue::Owned(format!("{:016x}", self.account_ref)),
        )
        .field("authority_revision", revision)
        .field("reason", FieldValue::Token(reason))
        .field(
            "elapsed_ms",
            FieldValue::U64(u64::try_from(self.received.elapsed().as_millis()).unwrap_or(u64::MAX)),
        )
    }

    pub(super) fn validation(&mut self, reason: &str) {
        self.awaiting_result = true;
        emit(
            "oteryn-game-server",
            &self.fields(
                Line::new(Level::Info, "social", "premium_snapshot_validation"),
                reason,
            ),
        );
    }

    pub(super) fn result(&mut self, reason: &str, error: Option<&DurabilityError>) {
        self.awaiting_result = false;
        let mut line = self.fields(
            Line::new(Level::Info, "social", "premium_snapshot_result"),
            reason,
        );
        if let Some(error) = error {
            // DurabilityError is already redacted and retains the typed database error class.
            line = line.field("error", FieldValue::Owned(format!("{error:?}")));
        }
        emit("oteryn-game-server", &line);
    }

    fn retry(&self, attempt: u64) {
        emit(
            "oteryn-game-server",
            &self
                .fields(
                    Line::new(Level::Info, "social", "premium_snapshot_retry"),
                    "holder_unavailable",
                )
                .field("attempt", FieldValue::U64(attempt)),
        );
    }
}

impl Drop for SnapshotTrace {
    fn drop(&mut self) {
        if self.awaiting_result {
            // Logout can cancel an in-flight pass before the durable outcome
            // is observed. Record the cancellation without guessing the outcome.
            self.result("cancelled", None);
        }
    }
}

/// A RootUnavailable result from the Premium write methods means checkout failed
/// before the transaction started. After checkout the pass owns its usual deadline
/// and commit classification. Never retry a timed-out or unknown write here.
pub(super) async fn retry_checkout<T, F>(
    root: &DurabilityRoot,
    trace: &SnapshotTrace,
    mut write: impl FnMut() -> F,
) -> Result<T, DurabilityError>
where
    F: Future<Output = Result<T, DurabilityError>>,
{
    let first = write().await;
    if !matches!(first, Err(DurabilityError::RootUnavailable)) {
        return first;
    }
    trace.retry(1);
    let deadline = Instant::now() + CHECKOUT_BUDGET;
    let _foreground = root.foreground_demand()?;
    let _waiter = root.holder_waiter();
    let mut attempt = 1;
    loop {
        let now = Instant::now();
        let poll = (now + HOLDER_POLL).min(deadline);
        if !root.wait_holder_turn(now, poll).await {
            tokio::time::sleep_until(poll).await;
        }
        if Instant::now() >= deadline {
            return Err(DurabilityError::RootUnavailable);
        }
        match write().await {
            Err(DurabilityError::RootUnavailable) => {
                attempt += 1;
                trace.retry(attempt);
            }
            result => return result,
        }
    }
}
