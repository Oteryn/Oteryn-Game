//! PREM-1a and PREM-1b: the Premium entitlement consumer (PREMIUM-ACTIVATION-V1 §4.1 as amended by
//! PREMIUM-DELIVERY-0; PROD-ENTITLEMENTS-01 consumer contract §6-§12).
//!
//! Platform is the only Premium authority. [`PremiumConsumer::ingest`] validates one snapshot
//! response ([`snapshot`]), fences it durably
//! ([`DurabilityRoot::accept_premium_evidence`]) and only then updates the in-memory view that
//! the gameplay reads use:
//!
//! - [`PremiumConsumer::premium_current`]: true only for [`PremiumClass::CurrentAuthority`];
//!   every other class reads as Free. Every Premium check in Game reads it.
//! - [`PremiumConsumer::premium_entitlement_ended`]: the entitlement itself ended; the only
//!   input of the login relocation (PREMIUM-ACTIVATION §4.5 as amended). A lapsed lease, missing
//!   evidence or a failed pull never makes it true.
//!
//! After a process start, durable evidence ([`PremiumConsumer::load`]) keeps its restrictive
//! facts and any durable semantic conflict, but an `ACTIVE` snapshot authorizes nothing until a
//! snapshot is fenced or replayed in this process (consumer contract §6.3).
//!
//! PREM-1b adds the transport (PREMIUM-DELIVERY-0 §3, §3.1): [`client`] pulls one snapshot over
//! mutual TLS, [`refresh`] schedules the admission, reconnect and refresh pulls, and a failed
//! pull marks the account `AUTHORITY_UNAVAILABLE` until a later pull succeeds
//! ([`PremiumConsumer::pull_failed`]).

pub mod client;
pub mod refresh;
pub mod snapshot;
#[cfg(test)]
pub mod test_producer;

use crate::durability::premium_fence::{
    EntitlementState, PremiumAccountRecord, PremiumEvidence, PremiumFenceOutcome, PremiumFenceView,
};
use crate::durability::{DurabilityError, DurabilityRoot};
use snapshot::{SnapshotFailure, SnapshotRejection};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

pub const SNAPSHOT_SCHEMA: &str = crate::durability::premium_fence::EVIDENCE_SCHEMA;
pub const PRODUCT_ID: &str = "oteryn.premium_time";
pub const PRODUCT_VERSION: u32 = 1;

/// The compatibility record of this consumer (consumer contract §4; PREMIUM-DELIVERY-0 §4): the
/// one (`producer_profile`, `product_version`) pair it accepts, under consumer policy
/// [`CONSUMER_POLICY_REVISION`] and the requested product policy (§5).
pub const PRODUCER_PROFILE: &str = "oteryn.entitlement.profile_b.v1";
pub const CONSUMER_POLICY_REVISION: &str = "premium-surfaces-1";
/// `max_authority_lease` (§5): a longer lease is outside the bound product policy.
pub const MAX_AUTHORITY_LEASE_US: i64 = 60 * 60 * 1_000_000;
/// `max_clock_skew` (§5).
pub const MAX_CLOCK_SKEW_US: i64 = 5 * 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    FreshAdmission,
    ReconnectOrRecovery,
    RunningSession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfacePolicy {
    RequireCurrent,
    AllowProducerStale,
}

/// PREMIUM-ACTIVATION §4.1, policy revision `premium-surfaces-1`: every surface requires current
/// authority, so no Premium read has a stale class and [`PremiumConsumer::premium_current`]
/// serves them all. Degraded (Free) behaviour is owned by the child that builds each benefit
/// (PREM-2..5).
pub const fn surface_policy(surface: Surface) -> SurfacePolicy {
    match surface {
        Surface::FreshAdmission | Surface::ReconnectOrRecovery | Surface::RunningSession => {
            SurfacePolicy::RequireCurrent
        }
    }
}

/// Consumer classifications (consumer contract §8.3). `NoEntitlement` is a `NONE` snapshot.
/// `StaleWithinBound` is kept evidence from its `refresh_after` until `authority_valid_until`;
/// the product policy permits no stale use, so it denies benefit like every class but
/// `CurrentAuthority` (Platform PREM-P §3, §8.2; PREMIUM-DELIVERY-0 §12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PremiumClass {
    CurrentAuthority,
    StaleWithinBound,
    NotYetEffective,
    Expired,
    Revoked,
    NoEntitlement,
    AuthorityUnavailable,
    InvalidOrConflicting,
}

/// Trusted server time `[lower, upper]` in Unix microseconds (consumer contract §7). The caller
/// supplies it from a synchronized clock; an unsynchronized node passes `None` to the reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrustedNow {
    lower_us: i64,
    upper_us: i64,
}

impl TrustedNow {
    /// `None` when the uncertainty exceeds `max_clock_skew`: Premium is then not current.
    pub fn new(now_us: i64, uncertainty_us: i64) -> Option<Self> {
        (0..=MAX_CLOCK_SKEW_US)
            .contains(&uncertainty_us)
            .then(|| Self {
                lower_us: now_us.saturating_sub(uncertainty_us),
                upper_us: now_us.saturating_add(uncertainty_us),
            })
    }
}

/// The ticket of one pull, taken when it starts: increasing across accounts and `release`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PullTicket(u64);

#[derive(Debug, Clone, Default)]
struct AccountView {
    fence: Option<PremiumFenceView>,
    /// A snapshot was fenced or replayed at the high water in this process.
    proven: bool,
    /// A semantic conflict (§3.1) is recorded for the account: sticky and never ignored.
    conflicting: bool,
    /// Validated evidence could not be fenced: the fence is unsafe (§6.4,
    /// `INVALID_OR_CONFLICTING`) until a later pull is fenced.
    quarantined: bool,
    /// A pull failed (§3.1): `AUTHORITY_UNAVAILABLE` until a later pull succeeds.
    unavailable: bool,
    /// The tickets of the latest quarantine, failed pull and proof. Only proof from a pull
    /// started after a failure clears it, and a failure from a pull started before the latest
    /// proof changes nothing, whatever order concurrent pulls finish in.
    quarantined_at: u64,
    unavailable_at: u64,
    proven_at: u64,
}

impl AccountView {
    fn quarantine(&mut self, ticket: PullTicket) {
        if ticket.0 > self.proven_at {
            self.quarantined = true;
            self.quarantined_at = self.quarantined_at.max(ticket.0);
        }
    }

    fn fail_pull(&mut self, ticket: PullTicket) {
        if ticket.0 > self.proven_at {
            self.unavailable = true;
            self.unavailable_at = self.unavailable_at.max(ticket.0);
        }
    }

    fn prove(&mut self, ticket: PullTicket, fence: PremiumFenceView) {
        merge(self, Some(fence));
        self.proven = true;
        self.proven_at = self.proven_at.max(ticket.0);
        if ticket.0 > self.quarantined_at {
            self.quarantined = false;
        }
        if ticket.0 > self.unavailable_at {
            self.unavailable = false;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestOutcome {
    Accepted,
    Replayed,
    Stale,
    Conflict,
    Rejected(SnapshotRejection),
    /// The fence could not be written or read; the account is quarantined.
    FenceUnavailable,
}

#[derive(Debug, Default)]
pub struct PremiumConsumer {
    accounts: Mutex<HashMap<[u8; 16], AccountView>>,
    /// Ingest tickets, increasing across accounts and `release`.
    tickets: AtomicU64,
}

impl PremiumConsumer {
    /// Load the durable fence and semantic conflict of `account_id`, before any Premium read at
    /// admission. An account already in memory keeps its view.
    pub async fn load(
        &self,
        root: &DurabilityRoot,
        account_id: [u8; 16],
    ) -> Result<(), DurabilityError> {
        let PremiumAccountRecord { fence, conflicting } =
            root.load_premium_fence(account_id).await?;
        self.update(account_id, |view| {
            merge(view, fence);
            view.conflicting |= conflicting;
        });
        Ok(())
    }

    /// The ticket of a pull that starts now.
    pub fn ticket(&self) -> PullTicket {
        PullTicket(self.tickets.fetch_add(1, Ordering::Relaxed) + 1)
    }

    /// A pull of `account_id` started at `ticket` yielded no response to ingest (§3.1 "anything
    /// else is unavailable"): the account is `AUTHORITY_UNAVAILABLE` until a later pull succeeds.
    pub fn pull_failed(&self, account_id: [u8; 16], ticket: PullTicket) {
        self.update(account_id, |view| view.fail_pull(ticket));
    }

    /// Validate, fence and apply one snapshot response to a request for `account_id` with
    /// `nonce`, as a pull that starts now.
    pub async fn ingest(
        &self,
        root: &DurabilityRoot,
        account_id: [u8; 16],
        nonce: &str,
        body: &[u8],
    ) -> IngestOutcome {
        let ticket = self.ticket();
        self.ingest_pull(root, account_id, ticket, nonce, body)
            .await
    }

    /// Validate, fence and apply the response of the pull started at `ticket`. A malformed or
    /// stale response is a failed pull; an unsupported one or a contradiction is a durable
    /// semantic conflict, recorded before this returns (§3.1).
    pub async fn ingest_pull(
        &self,
        root: &DurabilityRoot,
        account_id: [u8; 16],
        ticket: PullTicket,
        nonce: &str,
        body: &[u8],
    ) -> IngestOutcome {
        let evidence = match snapshot::validate(body, account_id, nonce) {
            Ok(evidence) => evidence,
            Err(SnapshotFailure::Malformed) => {
                self.pull_failed(account_id, ticket);
                return IngestOutcome::Rejected(SnapshotRejection::Malformed);
            }
            Err(SnapshotFailure::Unsupported(failure)) => {
                // Never ignored, whatever the ticket order: it denies Premium in this process
                // even when the durable record could not be written.
                self.update(account_id, |view| view.conflicting = true);
                return match root.record_premium_semantic_failure(&failure).await {
                    Ok(()) => IngestOutcome::Rejected(SnapshotRejection::Unsupported),
                    Err(_) => IngestOutcome::FenceUnavailable,
                };
            }
        };
        let Ok(outcome) = root.accept_premium_evidence(&evidence).await else {
            self.update(account_id, |view| view.quarantine(ticket));
            return IngestOutcome::FenceUnavailable;
        };
        self.update(account_id, |view| match outcome.clone() {
            PremiumFenceOutcome::Accepted(fence) | PremiumFenceOutcome::Replayed(fence) => {
                view.prove(ticket, fence);
            }
            PremiumFenceOutcome::Stale(fence) => {
                merge(view, Some(fence));
                view.fail_pull(ticket);
            }
            PremiumFenceOutcome::Conflict(fence) => {
                merge(view, Some(fence));
                view.conflicting = true;
            }
        });
        match outcome {
            PremiumFenceOutcome::Accepted(_) => IngestOutcome::Accepted,
            PremiumFenceOutcome::Replayed(_) => IngestOutcome::Replayed,
            PremiumFenceOutcome::Stale(_) => IngestOutcome::Stale,
            PremiumFenceOutcome::Conflict(_) => IngestOutcome::Conflict,
        }
    }

    /// Drop the in-memory view, when the account has no session left on this node.
    pub fn release(&self, account_id: [u8; 16]) {
        self.lock().remove(&account_id);
    }

    /// The `refresh_after` of the account's kept evidence: when the next pull is due.
    pub fn refresh_after_us(&self, account_id: [u8; 16]) -> Option<i64> {
        let accounts = self.lock();
        let fence = accounts.get(&account_id)?.fence.as_ref()?;
        Some(fence.latest.refresh_after_us)
    }

    pub fn class(&self, account_id: [u8; 16], now: Option<TrustedNow>) -> PremiumClass {
        classify(self.lock().get(&account_id), now)
    }

    /// The one Premium read of Game (PREMIUM-DELIVERY-0 §6).
    pub fn premium_current(&self, account_id: [u8; 16], now: Option<TrustedNow>) -> bool {
        self.class(account_id, now) == PremiumClass::CurrentAuthority
    }

    /// True only when the latest accepted evidence says the entitlement itself ended: producer
    /// `EXPIRED`, `REVOKED` or `NONE`, or `effective_until` surely passed. False without
    /// evidence or without trusted time for a time-based end. It reads only the kept evidence:
    /// a failed pull or a semantic failure neither makes it true nor clears it (§3.1, §6).
    pub fn premium_entitlement_ended(&self, account_id: [u8; 16], now: Option<TrustedNow>) -> bool {
        let accounts = self.lock();
        let Some(fence) = accounts
            .get(&account_id)
            .and_then(|view| view.fence.as_ref())
        else {
            return false;
        };
        let latest = &fence.latest;
        matches!(
            latest.state,
            EntitlementState::Expired | EntitlementState::Revoked | EntitlementState::None
        ) || now.is_some_and(|now| now.lower_us >= latest.effective_until_us)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<[u8; 16], AccountView>> {
        self.accounts.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn update(&self, account_id: [u8; 16], change: impl FnOnce(&mut AccountView)) {
        change(self.lock().entry(account_id).or_default());
    }
}

/// Apply a durable view without ever lowering the in-memory high water: concurrent results may
/// arrive out of order. A conflict marker is sticky.
fn merge(view: &mut AccountView, fence: Option<PremiumFenceView>) {
    let Some(fence) = fence else { return };
    let conflicting = fence.conflicting || view.fence.as_ref().is_some_and(|f| f.conflicting);
    let newer = view
        .fence
        .as_ref()
        .is_none_or(|current| fence.latest.authority_revision >= current.latest.authority_revision);
    if newer {
        view.fence = Some(fence);
    }
    if let Some(current) = view.fence.as_mut() {
        current.conflicting = conflicting;
    }
}

/// Consumer contract §8.1-§8.3 with restrictive precedence, for a `REQUIRE_CURRENT` surface.
fn classify(view: Option<&AccountView>, now: Option<TrustedNow>) -> PremiumClass {
    let Some(view) = view else {
        return PremiumClass::AuthorityUnavailable;
    };
    if view.conflicting || view.quarantined || view.fence.as_ref().is_some_and(|f| f.conflicting) {
        return PremiumClass::InvalidOrConflicting;
    }
    let Some(fence) = &view.fence else {
        return PremiumClass::AuthorityUnavailable;
    };
    let e: &PremiumEvidence = &fence.latest;
    match e.state {
        EntitlementState::Revoked => return PremiumClass::Revoked,
        EntitlementState::None => return PremiumClass::NoEntitlement,
        EntitlementState::Expired => return PremiumClass::Expired,
        EntitlementState::NotYetEffective | EntitlementState::Active => {}
    }
    if let Some(now) = now {
        if now.upper_us >= e.effective_until_us || now.upper_us >= e.authority_valid_until_us {
            return PremiumClass::Expired;
        }
        if now.lower_us < e.effective_from_us {
            return PremiumClass::NotYetEffective;
        }
    }
    if e.state == EntitlementState::NotYetEffective {
        return PremiumClass::NotYetEffective;
    }
    // Benefit ends at `refresh_after` without newer accepted evidence (PREM-P §8.2).
    if now.is_some_and(|now| now.upper_us >= e.refresh_after_us) {
        return PremiumClass::StaleWithinBound;
    }
    // A failed pull denies at once, even inside the kept interval (§3.1).
    if now.is_none() || !view.proven || view.unavailable {
        return PremiumClass::AuthorityUnavailable;
    }
    PremiumClass::CurrentAuthority
}

#[cfg(test)]
mod tests;
