//! PREM-1a: the Premium entitlement consumer (PREMIUM-ACTIVATION-V1 §4.1 as amended by
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
//! facts, but an `ACTIVE` snapshot authorizes nothing until a snapshot is fenced or replayed in
//! this process (consumer contract §6.3). The snapshot transport (mTLS pull, nonce, refresh
//! scheduling) is PREM-1b.

pub mod snapshot;

use crate::durability::premium_fence::{
    EntitlementState, PremiumEvidence, PremiumFenceOutcome, PremiumFenceView,
};
use crate::durability::{DurabilityError, DurabilityRoot};
use snapshot::SnapshotRejection;
use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

pub const SNAPSHOT_SCHEMA: &str = "oteryn.premium_snapshot.v1";
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

/// Consumer classifications (consumer contract §8.3). `STALE_WITHIN_BOUND` cannot occur: the
/// product policy permits no stale use (§5). `NoEntitlement` is a `NONE` snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PremiumClass {
    CurrentAuthority,
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

#[derive(Debug, Clone, Default)]
struct AccountView {
    fence: Option<PremiumFenceView>,
    /// A snapshot was fenced or replayed at the high water in this process.
    proven: bool,
    /// Validated evidence could not be fenced: deny until a later snapshot is.
    quarantined: bool,
    /// The latest bound response was outside the compatibility record.
    unsupported: bool,
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
}

impl PremiumConsumer {
    /// Load the durable fence of `account_id`, before any Premium read at admission. An account
    /// already in memory keeps its view.
    pub async fn load(
        &self,
        root: &DurabilityRoot,
        account_id: [u8; 16],
    ) -> Result<(), DurabilityError> {
        let fence = root.load_premium_fence(account_id).await?;
        self.update(account_id, |view| merge(view, fence));
        Ok(())
    }

    /// Validate, fence and apply one snapshot response to a request for `account_id` with
    /// `nonce`.
    pub async fn ingest(
        &self,
        root: &DurabilityRoot,
        account_id: [u8; 16],
        nonce: &str,
        body: &[u8],
    ) -> IngestOutcome {
        let evidence = match snapshot::validate(body, account_id, nonce) {
            Ok(evidence) => evidence,
            Err(rejection) => {
                if rejection == SnapshotRejection::Unsupported {
                    self.update(account_id, |view| view.unsupported = true);
                }
                return IngestOutcome::Rejected(rejection);
            }
        };
        let Ok(outcome) = root.accept_premium_evidence(&evidence).await else {
            self.update(account_id, |view| view.quarantined = true);
            return IngestOutcome::FenceUnavailable;
        };
        self.update(account_id, |view| match outcome.clone() {
            PremiumFenceOutcome::Accepted(fence) | PremiumFenceOutcome::Replayed(fence) => {
                merge(view, Some(fence));
                view.proven = true;
                view.quarantined = false;
                view.unsupported = false;
            }
            PremiumFenceOutcome::Stale(fence) | PremiumFenceOutcome::Conflict(fence) => {
                merge(view, Some(fence));
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

    pub fn class(&self, account_id: [u8; 16], now: Option<TrustedNow>) -> PremiumClass {
        classify(self.lock().get(&account_id), now)
    }

    /// The one Premium read of Game (PREMIUM-DELIVERY-0 §6).
    pub fn premium_current(&self, account_id: [u8; 16], now: Option<TrustedNow>) -> bool {
        self.class(account_id, now) == PremiumClass::CurrentAuthority
    }

    /// True only when the latest accepted evidence says the entitlement itself ended: producer
    /// `EXPIRED`, `REVOKED` or `NONE`, or `effective_until` surely passed. False without
    /// evidence, with conflicting or unsupported evidence, or without trusted time for a
    /// time-based end.
    pub fn premium_entitlement_ended(&self, account_id: [u8; 16], now: Option<TrustedNow>) -> bool {
        let accounts = self.lock();
        let Some(view) = accounts.get(&account_id) else {
            return false;
        };
        let Some(fence) = view
            .fence
            .as_ref()
            .filter(|f| !f.conflicting && !view.unsupported)
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
    if view.unsupported || view.fence.as_ref().is_some_and(|f| f.conflicting) {
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
    if now.is_none() || view.quarantined || !view.proven {
        return PremiumClass::AuthorityUnavailable;
    }
    PremiumClass::CurrentAuthority
}

#[cfg(test)]
mod tests;
