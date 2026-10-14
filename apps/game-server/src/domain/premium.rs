//! Pure promotion-benefit rules plus the 2026-10-01 owner soul amendment.
//! Premium now selects soul maximum independently of durable promotion; expiry clamps soul.
//!
//! The caller supplies whether Premium is current (PREM-1) and the durable promotion state; the
//! regeneration schedule and soul sources are separate children.
//!
//! PREM-WIRE-1: [`PremiumGate`] is the one value a Premium-gated decision reads (PREMIUM-ACTIVATION
//! §4.1: every surface `REQUIRE_CURRENT`, every other class reads as Free). It starts closed, and
//! only a Premium fence read that classified the account as current authority opens it.

/// Soul maximum without a current promotion benefit.
pub const BASE_SOUL_MAXIMUM: u32 = 100;
/// Soul maximum while authenticated Premium is current (owner amendment 2026-10-01).
pub const PROMOTED_SOUL_MAXIMUM: u32 = 200;

/// D76: a promotion benefit applies only while the durable promotion is set and Premium is current
/// at the moment of use. The durable promotion itself never changes because Premium lapsed.
#[must_use]
pub const fn promotion_benefit_current(promoted: bool, premium_current: bool) -> bool {
    promoted && premium_current
}

/// The current account type selects the soul maximum at every transition and gain.
#[must_use]
pub const fn soul_maximum(premium_current: bool) -> u32 {
    if premium_current {
        PROMOTED_SOUL_MAXIMUM
    } else {
        BASE_SOUL_MAXIMUM
    }
}

/// Apply an authenticated grant change, expiry, revocation or unavailable benefit.
/// Raising the maximum does not refill soul; lowering it clamps immediately.
#[must_use]
pub const fn apply_account_transition(current: u32, premium_current: bool) -> u32 {
    let maximum = soul_maximum(premium_current);
    if current > maximum { maximum } else { current }
}

/// A gain uses the same current maximum and never preserves out-of-bound soul.
#[must_use]
pub const fn apply_soul_gain(current: u32, gain: u32, premium_current: bool) -> u32 {
    let maximum = soul_maximum(premium_current);
    let current = apply_account_transition(current, premium_current);
    let raised = current.saturating_add(gain);
    if raised > maximum { maximum } else { raised }
}

/// Why a Premium-gated decision was refused: Premium is not current. One reason on purpose: a
/// lapsed, unknown, unavailable or conflicting account all read as Free (§4.1 amendment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PremiumRefusal {
    NotCurrent,
}

/// The account's Premium standing at one decision point. Fail closed: [`Default`] and
/// [`PremiumGate::CLOSED`] refuse, and only a Premium fence owner opens it from a read that
/// classified the account as current authority under trusted time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PremiumGate {
    current: bool,
}

impl PremiumGate {
    /// No fence read, or one that did not prove current authority.
    pub const CLOSED: Self = Self { current: false };

    /// For the Premium fence owners only: `current` is their read of current authority.
    pub(crate) const fn from_fence_read(current: bool) -> Self {
        Self { current }
    }

    /// The `premium_current` input of the rules above.
    #[must_use]
    pub const fn premium_current(self) -> bool {
        self.current
    }

    /// Admit a Premium-gated action (a Premium spell, a Premium-area entry, a Premium purchase).
    pub const fn require(self) -> Result<(), PremiumRefusal> {
        if self.current {
            Ok(())
        } else {
            Err(PremiumRefusal::NotCurrent)
        }
    }
}

/// Proof that Premium has not been delivered on this deployment, valid only inside the Premium
/// gate (PREMIUM-ACTIVATION-0 §1.2). It is neither `Copy` nor `Clone` and borrows the gate, so a
/// pre-delivery bypass (GUILD-0 §3.2-§3.3, WHEEL-0 §6.2) can only be applied while the gate holds
/// the shared latch lock.
#[derive(Debug)]
pub struct PreDelivery<'g> {
    _gate: std::marker::PhantomData<&'g mut ()>,
}

impl PreDelivery<'_> {
    /// For the Premium gate only, inside its shared-lock transaction.
    pub(crate) const fn inside_gate() -> Self {
        Self {
            _gate: std::marker::PhantomData,
        }
    }
}

/// The account's Premium status at one gated command (PREMIUM-ACTIVATION-0 §1.2). A consumer with
/// no pre-delivery rule treats `NotActivated` as `NotCurrent`.
#[derive(Debug)]
pub enum PremiumStatus<'g> {
    /// Premium has not started on this deployment.
    NotActivated(PreDelivery<'g>),
    /// Premium has started and the account's Premium is current.
    Current,
    /// Premium has started (or its start is unknown) and the account reads Free.
    NotCurrent,
}

impl PremiumStatus<'_> {
    /// The gate of a consumer with no pre-delivery rule: open only for `Current`.
    #[must_use]
    pub const fn gate(&self) -> PremiumGate {
        PremiumGate::from_fence_read(matches!(self, Self::Current))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_current_status_opens_the_gate() {
        assert!(PremiumStatus::Current.gate().premium_current());
        assert_eq!(PremiumStatus::NotCurrent.gate(), PremiumGate::CLOSED);
        assert_eq!(
            PremiumStatus::NotActivated(PreDelivery::inside_gate()).gate(),
            PremiumGate::CLOSED
        );
    }

    #[test]
    fn the_premium_gate_is_closed_unless_a_fence_read_proved_current_authority() {
        for closed in [
            PremiumGate::default(),
            PremiumGate::CLOSED,
            PremiumGate::from_fence_read(false),
        ] {
            assert!(!closed.premium_current());
            assert_eq!(closed.require(), Err(PremiumRefusal::NotCurrent));
            assert_eq!(soul_maximum(closed.premium_current()), BASE_SOUL_MAXIMUM);
            assert!(!promotion_benefit_current(true, closed.premium_current()));
        }
        let open = PremiumGate::from_fence_read(true);
        assert_eq!(open.require(), Ok(()));
        assert_eq!(soul_maximum(open.premium_current()), PROMOTED_SOUL_MAXIMUM);
    }

    #[test]
    fn the_promotion_benefit_needs_both_promotion_and_current_premium() {
        assert!(promotion_benefit_current(true, true));
        assert!(!promotion_benefit_current(true, false));
        assert!(!promotion_benefit_current(false, true));
        assert!(!promotion_benefit_current(false, false));
    }

    #[test]
    fn the_soul_maximum_follows_the_current_account_type() {
        assert_eq!(soul_maximum(true), 200);
        assert_eq!(soul_maximum(false), 100);
        assert_eq!(soul_maximum(promotion_benefit_current(true, false)), 100);
    }

    #[test]
    fn a_gain_is_capped_exactly_at_the_maximum() {
        assert_eq!(apply_soul_gain(95, 4, false), 99);
        assert_eq!(apply_soul_gain(95, 5, false), 100);
        assert_eq!(apply_soul_gain(95, 6, false), 100);
        assert_eq!(apply_soul_gain(100, 1, false), 100);
        assert_eq!(apply_soul_gain(150, 60, true), 200);
        assert_eq!(apply_soul_gain(0, u32::MAX, true), 200);
        assert_eq!(apply_soul_gain(10, 0, false), 10);
    }

    #[test]
    fn premium_expiry_clamps_soul_and_grant_does_not_refill() {
        assert_eq!(apply_account_transition(200, false), 100);
        assert_eq!(apply_account_transition(101, false), 100);
        assert_eq!(apply_account_transition(50, true), 50);
        assert_eq!(apply_soul_gain(200, 5, false), 100);
        assert_eq!(apply_soul_gain(101, 10, false), 100);
    }

    #[test]
    fn regeneration_never_exceeds_the_maximum() {
        for promoted in [false, true] {
            let maximum = soul_maximum(promoted);
            let mut soul = 0;
            for _ in 0..500 {
                soul = apply_soul_gain(soul, 1, promoted);
                assert!(soul <= maximum);
            }
            assert_eq!(soul, maximum);
        }
    }
}
