//! Pure promotion-benefit rules plus the 2026-10-01 owner soul amendment.
//! Premium now selects soul maximum independently of durable promotion; expiry clamps soul.
//!
//! The caller supplies whether Premium is current (PREM-1) and the durable promotion state; the
//! regeneration schedule and soul sources are separate children.

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

#[cfg(test)]
mod tests {
    use super::*;

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
