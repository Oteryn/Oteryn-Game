//! PREM-1b: the Premium pull schedule (PREMIUM-DELIVERY-0 §3, §3.1; §11 scope item 4).
//!
//! [`PremiumRefresher::admit`] starts, or wakes, one task per online account: it loads the
//! durable fence, pulls at once (fresh admission or reconnect), then pulls again
//! [`REFRESH_LEAD`] before each snapshot's `refresh_after`, never sooner than
//! [`MIN_REFRESH_INTERVAL`] after the last successful pull. Benefit ends at `refresh_after`
//! without newer evidence (Platform PREM-P §8.2, §8.5), so the lead leaves room for retries
//! before it. A failed pull is retried with capped exponential backoff and jitter; a 429 or 503
//! `Retry-After` is honoured within the cap. The one task per account is the only puller, so
//! at most one request is in flight per account. Fresh admission may await that same
//! scheduled pull before applying the account's Premium state. [`PremiumRefresher::release`]
//! cancels the schedule and drops the in-memory view.

use super::client::{PULL_TIMEOUT, PremiumSnapshotClient, fresh_nonce};
use super::{IngestOutcome, PremiumConsumer, PullTicket, diagnostics::SnapshotTrace};
use crate::durability::DurabilityRoot;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{Notify, watch};
use tokio::task::JoinHandle;

/// The floor between a successful pull and the next scheduled one (§10.2), so an authenticated
/// producer cannot drive a request loop.
pub const MIN_REFRESH_INTERVAL: Duration = Duration::from_secs(60);
/// How long before `refresh_after` the scheduled pull starts (PREM-P §8.5): several capped
/// retries fit in it.
pub const REFRESH_LEAD: Duration = Duration::from_secs(5 * 60);
pub const RETRY_BASE: Duration = Duration::from_secs(1);
/// The backoff cap, which also bounds an honoured `Retry-After`.
pub const RETRY_CAP: Duration = Duration::from_secs(60);

/// The delay after a successful pull: until [`REFRESH_LEAD`] before `refresh_after`, at least
/// [`MIN_REFRESH_INTERVAL`].
pub fn after_success(refresh_after_us: i64, now_us: i64) -> Duration {
    let lead_us = i64::try_from(REFRESH_LEAD.as_micros()).unwrap_or(i64::MAX);
    let until = u64::try_from(
        refresh_after_us
            .saturating_sub(lead_us)
            .saturating_sub(now_us),
    )
    .unwrap_or(0);
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
    Retry(Option<Duration>, SnapshotWaitFailure),
}

/// Why the scheduled pull cannot supply fenced evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotWaitFailure {
    NotConfigured,
    ScheduleMissing,
    ScheduleStopped,
    Deadline,
    AuthorityUnavailable,
    SnapshotRejected,
    FenceUnavailable,
}

struct Schedule {
    wake: Arc<Notify>,
    task: JoinHandle<()>,
    snapshot: watch::Receiver<Option<Result<(), SnapshotWaitFailure>>>,
}

struct Inner {
    consumer: Arc<PremiumConsumer>,
    root: DurabilityRoot,
    client: Option<PremiumSnapshotClient>,
}

/// The scheduled tasks. Owned by the refresher handles only, never by a task, so dropping the
/// last handle (a node shutdown) aborts every task.
#[derive(Default)]
struct Schedules(Mutex<HashMap<[u8; 16], Schedule>>);

impl Drop for Schedules {
    fn drop(&mut self) {
        let schedules = self.0.get_mut().unwrap_or_else(PoisonError::into_inner);
        for (_, schedule) in schedules.drain() {
            schedule.task.abort();
        }
    }
}

/// The Premium pulls of one node. Without a client (no configuration) nothing is pulled and
/// every account reads Free. Dropping the last handle cancels every schedule.
#[derive(Clone)]
pub struct PremiumRefresher {
    inner: Arc<Inner>,
    schedules: Arc<Schedules>,
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
            }),
            schedules: Arc::default(),
        }
    }

    pub fn consumer(&self) -> &PremiumConsumer {
        &self.inner.consumer
    }

    /// Fresh admission or reconnect of `account_id`: pull before any Premium read. Returns at
    /// once; must run inside the Tokio runtime.
    pub fn admit(&self, account_id: [u8; 16]) {
        self.inner.consumer.profile_b_admit(account_id);
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
        self.inner
            .consumer
            .profile_b_register_wake(account_id, &wake);
        let task_wake = wake.clone();
        let inner = self.inner.clone();
        let (outcome, snapshot) = watch::channel(None);
        let task = oteryn_task_supervisor::spawn_supervised_with_heartbeat(
            "premium_refresh",
            None,
            move |heartbeat| run(inner, account_id, task_wake, outcome, heartbeat),
        );
        schedules.insert(
            account_id,
            Schedule {
                wake,
                task,
                snapshot,
            },
        );
    }

    /// Join the account's existing pull without issuing another Platform request.
    /// A temporary fence checkout failure leaves a cold reader waiting for the
    /// scheduled retry within the HTTP and durable recovery bounds.
    pub async fn wait_for_snapshot(&self, account_id: [u8; 16]) -> Result<(), SnapshotWaitFailure> {
        if self.inner.client.is_none() {
            return Err(SnapshotWaitFailure::NotConfigured);
        }
        let mut snapshot = self
            .schedules()
            .get(&account_id)
            .map(|schedule| schedule.snapshot.clone())
            .ok_or(SnapshotWaitFailure::ScheduleMissing)?;
        let outcome = tokio::time::timeout(
            PULL_TIMEOUT + crate::durability::ROOT_RECOVERY_WINDOW,
            snapshot.wait_for(|outcome| {
                outcome.is_some() && *outcome != Some(Err(SnapshotWaitFailure::FenceUnavailable))
            }),
        )
        .await;
        match outcome {
            Ok(Ok(outcome)) => (*outcome).ok_or(SnapshotWaitFailure::ScheduleStopped)?,
            Ok(Err(_)) => Err(SnapshotWaitFailure::ScheduleStopped),
            Err(_) => Err(SnapshotWaitFailure::Deadline),
        }
    }

    /// The account has no session left on this node: cancel its schedule and drop its view.
    pub fn release(&self, account_id: [u8; 16]) {
        self.inner.consumer.profile_b_release(account_id);
        if let Some(schedule) = self.schedules().remove(&account_id) {
            schedule.task.abort();
        }
        self.inner.consumer.release(account_id);
    }

    /// Whether an account has a live task that can provide a snapshot.
    pub fn has_schedule(&self, account_id: [u8; 16]) -> bool {
        self.schedules()
            .get(&account_id)
            .is_some_and(|schedule| !schedule.task.is_finished())
    }

    fn schedules(&self) -> std::sync::MutexGuard<'_, HashMap<[u8; 16], Schedule>> {
        self.schedules
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

async fn run(
    inner: Arc<Inner>,
    account_id: [u8; 16],
    wake: Arc<Notify>,
    outcome: watch::Sender<Option<Result<(), SnapshotWaitFailure>>>,
    heartbeat: oteryn_task_supervisor::Heartbeat,
) {
    let Some(client) = &inner.client else { return };
    // A load failure only delays the restrictive durable facts: the first pull fences against
    // the same durable rows and returns them.
    if let Err(error) = inner.consumer.load(&inner.root, account_id).await {
        SnapshotTrace::new(account_id).result("initial_fence_load_failed", Some(&error));
    }
    let mut attempt = 0;
    loop {
        heartbeat.beat();
        let ticket = inner.consumer.ticket();
        let delay = match pull(&inner, client, account_id, ticket).await {
            Next::Refresh => {
                outcome.send_replace(Some(Ok(())));
                attempt = 0;
                match (inner.consumer.refresh_after_us(account_id), now_us()) {
                    (Some(at), Some(now)) => after_success(at, now),
                    (Some(_), None) => {
                        SnapshotTrace::new(account_id).result("refresh_clock_unavailable", None);
                        MIN_REFRESH_INTERVAL
                    }
                    (None, _) => MIN_REFRESH_INTERVAL,
                }
            }
            Next::Retry(retry_after, reason) => {
                outcome.send_replace(Some(Err(reason)));
                let delay = after_failure(attempt, retry_after, jitter());
                attempt = attempt.saturating_add(1);
                delay
            }
        };
        // Sleep until the next pull is due, or a reconnect wakes it.
        let _idle = heartbeat.expected_idle();
        // Either a wake or normal timer expiry starts the next pull.
        tokio::time::timeout(delay, wake.notified())
            .await
            .unwrap_or(());
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
        SnapshotTrace::new(account_id).result("nonce_unavailable", None);
        return Next::Retry(None, SnapshotWaitFailure::AuthorityUnavailable);
    };
    let body = match client.pull(account_id, &nonce).await {
        Ok(body) => body,
        Err(failure) => {
            consumer.pull_failed(account_id, ticket);
            use super::client::PullFailure;
            let reason = match failure {
                PullFailure::Status { status, .. } => format!("http_{status}"),
                PullFailure::Transport => "transport".to_owned(),
                PullFailure::Timeout => "transport_deadline".to_owned(),
                PullFailure::ContentType => "content_type".to_owned(),
                PullFailure::Oversize => "oversize".to_owned(),
            };
            SnapshotTrace::new(account_id).result(&reason, None);
            return Next::Retry(
                failure.retry_after(),
                SnapshotWaitFailure::AuthorityUnavailable,
            );
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
        | IngestOutcome::Rejected(super::snapshot::SnapshotRejection::Malformed) => {
            Next::Retry(None, SnapshotWaitFailure::SnapshotRejected)
        }
        IngestOutcome::FenceUnavailable => Next::Retry(None, SnapshotWaitFailure::FenceUnavailable),
    }
}

fn now_us() -> Option<i64> {
    let Ok(elapsed) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return None;
    };
    i64::try_from(elapsed.as_micros()).ok()
}

pub(crate) fn jitter() -> f64 {
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
