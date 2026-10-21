//! Strict health thresholds using overflow-safe integer arithmetic.

/// Equality at the authored threshold does not start a heal.
pub fn below_health_threshold(health: u64, maximum: u64, threshold_ppm: u64) -> bool {
    u128::from(health) * 1000000 < u128::from(maximum) * u128::from(threshold_ppm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_threshold_and_wide_products() {
        assert!(!below_health_threshold(20, 100, 200000));
        assert!(below_health_threshold(19, 100, 200000));
        assert!(!below_health_threshold(21, 100, 200000));
        assert!(!below_health_threshold(u64::MAX, u64::MAX, 1000000));
        assert!(below_health_threshold(u64::MAX - 1, u64::MAX, 1000000));
        assert!(!below_health_threshold(0, u64::MAX, 0));
        assert!(below_health_threshold(0, u64::MAX, 1));
    }
}
