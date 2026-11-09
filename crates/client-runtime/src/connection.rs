//! Native session refusal and reconciliation retries, independent of Platform and transport.
use oteryn_foundation::{Cancellable, CancellationToken, cancellable};
use oteryn_session::{AdmissionPublicClass, AdmissionQueue, admission_refusal_class};
use std::future::Future;
use std::time::{Duration, Instant};

// N8 (packet §1.2) admission refusal codes the client retries.
const GRANT_NOT_YET_VALID: u32 = 1103;
const ATTEMPT_RECONCILIATION_REQUIRED: u32 = 1106;
const GRANT_SECURITY_EVIDENCE_STALE: u32 = 1108;
const CAPACITY_EXCEEDED: u32 = 1115;
/// Retries of the same grant on 1103/1108, and Gateway re-logins on 1106.
const MAX_GRANT_RETRIES: u32 = 3;

/// Observable retry timing; it carries no invented queue position or grant material.
#[derive(Debug, Clone, Copy)]
pub struct AdmissionWait {
    pub code: u32,
    pub started: Instant,
    pub retry_in: Duration,
    pub queue_position: Option<u32>,
}

/// Composition adapter for one grant and its transport. Reconciliation keeps the same login attempt.
pub trait AdmissionAttempt {
    type Output;
    type Error;

    fn admit(&mut self) -> impl Future<Output = Result<Self::Output, Option<u32>>>;
    fn reconcile(
        &mut self,
        cancellation: CancellationToken,
    ) -> impl Future<Output = Result<(), Self::Error>>;
    fn waiting(&mut self, _wait: AdmissionWait) {}
    fn queue(&self) -> Option<AdmissionQueue> {
        None
    }
    fn usable_until(&self) -> Instant;
    fn rejected(class: AdmissionPublicClass) -> Self::Error;
}

/// Connects and admits, retrying only the N8 RETRYABLE rows.
pub async fn enter<A: AdmissionAttempt>(
    attempt: &mut A,
    retry_unit: Duration,
    cancellation: CancellationToken,
) -> Result<A::Output, A::Error> {
    // 5-6. Connect and admit, retrying only the N8 RETRYABLE rows.
    let mut grant_retries = 0;
    let mut reconciliations = 0;
    let mut capacity_retried = false;
    let mut queued_retries = 0;
    loop {
        let code = match attempt.admit().await {
            Ok(session) => return Ok(session),
            Err(Some(code)) => code,
            Err(None) => return Err(A::rejected(AdmissionPublicClass::TemporarilyUnavailable)),
        };
        let class = admission_refusal_class(code).map_or(
            AdmissionPublicClass::TemporarilyUnavailable,
            |(_progression, class)| class,
        );
        let queue = (code == CAPACITY_EXCEEDED)
            .then(|| attempt.queue())
            .flatten();
        let delay = match code {
            CAPACITY_EXCEEDED if queue.is_some() && queued_retries < 16 => {
                queued_retries += 1;
                Duration::from_millis(u64::from(queue.map_or(0, |queue| queue.retry_after_ms)))
            }
            GRANT_NOT_YET_VALID | GRANT_SECURITY_EVIDENCE_STALE
                if grant_retries < MAX_GRANT_RETRIES =>
            {
                grant_retries += 1;
                retry_unit * (1 << (grant_retries - 1))
            }
            CAPACITY_EXCEEDED if queue.is_none() && !capacity_retried => {
                capacity_retried = true;
                retry_unit * 2
            }
            ATTEMPT_RECONCILIATION_REQUIRED if reconciliations < MAX_GRANT_RETRIES => {
                // The same attempt_ref and ticket: Platform returns the identical grant.
                reconciliations += 1;
                attempt.reconcile(cancellation.clone()).await?;
                continue;
            }
            _ => return Err(A::rejected(class)),
        };
        if Instant::now() + delay >= attempt.usable_until() {
            return Err(A::rejected(class));
        }
        attempt.waiting(AdmissionWait {
            code,
            started: Instant::now(),
            retry_in: delay,
            queue_position: queue.map(|queue| queue.position),
        });
        match cancellable(cancellation.clone(), tokio::time::sleep(delay)).await {
            Cancellable::Completed(()) => {}
            Cancellable::Cancelled => return Err(A::rejected(class)),
        }
    }
}
