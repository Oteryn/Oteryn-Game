//! PREM-1b: the Premium pull schedule (PREMIUM-DELIVERY-0 §3, §3.1; §11 scope item 4).
//!
//! [`PremiumRefresher::admit`] starts, or wakes, one task per online account: it loads the
//! durable fence, pulls at once (fresh admission or reconnect), then pulls again at each
//! snapshot's `refresh_after`, never sooner than [`MIN_REFRESH_INTERVAL`] after the last
//! successful pull. A failed pull is retried with capped exponential backoff and jitter; a 429 or
//! 503 `Retry-After` is honoured within the cap. The one task per account is the only puller, so
//! at most one request is in flight per account. Admission never waits on a pull: until one
//! succeeds the account reads as not current. [`PremiumRefresher::release`] cancels the
//! schedule and drops the in-memory view.

use super::client::{PremiumSnapshotClient, fresh_nonce};
use super::{IngestOutcome, PremiumConsumer, PullTicket};
use crate::durability::DurabilityRoot;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// The floor between a successful pull and the next scheduled one (§10.2), so an authenticated
/// producer cannot drive a request loop.
pub const MIN_REFRESH_INTERVAL: Duration = Duration::from_secs(60);
pub const RETRY_BASE: Duration = Duration::from_secs(1);
/// The backoff cap, which also bounds an honoured `Retry-After`.
pub const RETRY_CAP: Duration = Duration::from_secs(60);

/// The delay after a successful pull: until `refresh_after`, at least [`MIN_REFRESH_INTERVAL`].
pub fn after_success(refresh_after_us: i64, now_us: i64) -> Duration {
    let until = u64::try_from(refresh_after_us.saturating_sub(now_us)).unwrap_or(0);
    Duration::from_micros(until).max(MIN_REFRESH_INTERVAL)
}

/// The delay before retry `attempt` (0 for the first) after a failed pull: exponential from
/// [`RETRY_BASE`], capped at [`RETRY_CAP`], with equal jitter (`jitter` in `[0, 1]`); a
/// `Retry-After` lengthens it up to the cap.
pub fn after_failure(attempt: u32, retry_after: Option<Duration>, jitter: f64) -> Duration {
    let ceiling = RETRY_BASE
        .saturating_mul(1 << attempt.min(16))
        .min(RETRY_CAP);
    let backoff = ceiling / 2 + (ceiling / 2).mul_f64(jitter.clamp(0.0, 1.0));
    retry_after.map_or(backoff, |wait| backoff.max(wait.min(RETRY_CAP)))
}

/// What one pull leaves the schedule to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Next {
    /// The pull succeeded or ended in a durable denial: refresh on schedule.
    Refresh,
    /// The pull failed: retry with backoff.
    Retry(Option<Duration>),
}

struct Schedule {
    wake: Arc<Notify>,
    task: JoinHandle<()>,
}

struct Inner {
    consumer: Arc<PremiumConsumer>,
    root: DurabilityRoot,
    client: Option<PremiumSnapshotClient>,
    schedules: Mutex<HashMap<[u8; 16], Schedule>>,
}

/// The Premium pulls of one node. Without a client (no configuration) nothing is pulled and
/// every account reads Free.
#[derive(Clone)]
pub struct PremiumRefresher {
    inner: Arc<Inner>,
}

impl PremiumRefresher {
    pub fn new(
        consumer: Arc<PremiumConsumer>,
        root: DurabilityRoot,
        client: Option<PremiumSnapshotClient>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                consumer,
                root,
                client,
                schedules: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn consumer(&self) -> &PremiumConsumer {
        &self.inner.consumer
    }

    /// Fresh admission or reconnect of `account_id`: pull before any Premium read. Returns at
    /// once; must run inside the Tokio runtime.
    pub fn admit(&self, account_id: [u8; 16]) {
        if self.inner.client.is_none() {
            return;
        }
        let mut schedules = self.schedules();
        if let Some(schedule) = schedules.get(&account_id)
            && !schedule.task.is_finished()
        {
            // Pull now; a pull in flight is followed by exactly one more.
            schedule.wake.notify_one();
            return;
        }
        let wake = Arc::new(Notify::new());
        let task = tokio::spawn(run(self.inner.clone(), account_id, wake.clone()));
        schedules.insert(account_id, Schedule { wake, task });
    }

    /// The account has no session left on this node: cancel its schedule and drop its view.
    pub fn release(&self, account_id: [u8; 16]) {
        if let Some(schedule) = self.schedules().remove(&account_id) {
            schedule.task.abort();
        }
        self.inner.consumer.release(account_id);
    }

    fn schedules(&self) -> std::sync::MutexGuard<'_, HashMap<[u8; 16], Schedule>> {
        self.inner
            .schedules
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

async fn run(inner: Arc<Inner>, account_id: [u8; 16], wake: Arc<Notify>) {
    let Some(client) = &inner.client else { return };
    // A load failure only delays the restrictive durable facts: the first pull fences against
    // the same durable rows and returns them.
    let _ = inner.consumer.load(&inner.root, account_id).await;
    let mut attempt = 0;
    loop {
        let ticket = inner.consumer.ticket();
        let delay = match pull(&inner, client, account_id, ticket).await {
            Next::Refresh => {
                attempt = 0;
                let refresh_after = inner.consumer.refresh_after_us(account_id);
                refresh_after.map_or(MIN_REFRESH_INTERVAL, |at| after_success(at, now_us()))
            }
            Next::Retry(retry_after) => {
                let delay = after_failure(attempt, retry_after, jitter());
                attempt = attempt.saturating_add(1);
                delay
            }
        };
        // Sleep until the next pull is due, or a reconnect wakes it.
        let _ = tokio::time::timeout(delay, wake.notified()).await;
    }
}

async fn pull(
    inner: &Inner,
    client: &PremiumSnapshotClient,
    account_id: [u8; 16],
    ticket: PullTicket,
) -> Next {
    let consumer = &inner.consumer;
    let Some(nonce) = fresh_nonce() else {
        consumer.pull_failed(account_id, ticket);
        return Next::Retry(None);
    };
    let body = match client.pull(account_id, &nonce).await {
        Ok(body) => body,
        Err(failure) => {
            consumer.pull_failed(account_id, ticket);
            return Next::Retry(failure.retry_after());
        }
    };
    match consumer
        .ingest_pull(&inner.root, account_id, ticket, &nonce, &body)
        .await
    {
        IngestOutcome::Accepted
        | IngestOutcome::Replayed
        | IngestOutcome::Conflict
        | IngestOutcome::Rejected(super::snapshot::SnapshotRejection::Unsupported) => Next::Refresh,
        IngestOutcome::Stale
        | IngestOutcome::Rejected(super::snapshot::SnapshotRejection::Malformed)
        | IngestOutcome::FenceUnavailable => Next::Retry(None),
    }
}

fn now_us() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_micros()).ok())
        .unwrap_or(0)
}

fn jitter() -> f64 {
    let mut byte = [0u8; 2];
    let filled = rustls::crypto::aws_lc_rs::default_provider()
        .secure_random
        .fill(&mut byte)
        .is_ok();
    if filled {
        f64::from(u16::from_be_bytes(byte)) / f64::from(u16::MAX)
    } else {
        0.5
    }
}
