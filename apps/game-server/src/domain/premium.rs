//! Pure promotion-benefit and soul-maximum rules (`OTERYN_GAME_PREMIUM_ACTIVATION_DECISION`
//! §4.2-§4.3, owner decisions D72, D74 and D76).
//!
//! The caller supplies whether Premium is current (PREM-1) and the durable promotion state; the
//! regeneration schedule and soul sources are separate children.

/// Soul maximum without a current promotion benefit.
pub const BASE_SOUL_MAXIMUM: u32 = 100;
/// Soul maximum while the promotion benefit is current (D72).
pub const PROMOTED_SOUL_MAXIMUM: u32 = 200;

/// D76: a promotion benefit applies only while the durable promotion is set and Premium is current
/// at the moment of use. The durable promotion itself never changes because Premium lapsed.
#[must_use]
pub const fn promotion_benefit_current(promoted: bool, premium_current: bool) -> bool {
    promoted && premium_current
}

/// D72/D76: the soul maximum at a gain or regeneration step.
#[must_use]
pub const fn soul_maximum(promotion_benefit_current: bool) -> u32 {
    if promotion_benefit_current {
        PROMOTED_SOUL_MAXIMUM
    } else {
        BASE_SOUL_MAXIMUM
    }
}

/// A soul gain or regeneration step. It raises soul to at most the current maximum; soul already
/// above the maximum after a demotion is kept until spent and never clamped down (D74).
#[must_use]
pub const fn apply_soul_gain(current: u32, gain: u32, promotion_benefit_current: bool) -> u32 {
    let maximum = soul_maximum(promotion_benefit_current);
    if current >= maximum {
        return current;
    }
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
    fn the_soul_maximum_follows_the_current_promotion_benefit() {
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
    fn soul_above_the_maximum_after_a_demotion_is_kept_until_spent() {
        // 200 soul earned while promoted is kept after Premium lapsed; a gain adds nothing.
        assert_eq!(apply_soul_gain(200, 5, false), 200);
        assert_eq!(apply_soul_gain(101, 10, false), 101);
        // After spending below the maximum, gains resume up to the maximum only.
        let after_spend = 200 - 150;
        assert_eq!(apply_soul_gain(after_spend, 80, false), 100);
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
