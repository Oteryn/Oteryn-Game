//! Deadline arithmetic on caller-supplied ticks, without a runtime clock.

/// Refuse an unrepresentable deadline.
pub fn checked_deadline(now: u64, delay: u64) -> Option<u64> {
    now.checked_add(delay)
}

/// Saturating add: a deadline computation never wraps or panics on overflow.
pub fn saturating_deadline(now: u64, delay: u64) -> u64 {
    now.saturating_add(delay)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_deadlines_and_distinct_overflow_policies() {
        assert_eq!(checked_deadline(1, 30), Some(31));
        assert_eq!(saturating_deadline(1, 30), 31);
        assert_eq!(checked_deadline(u64::MAX - 1, 1), Some(u64::MAX));
        assert_eq!(saturating_deadline(u64::MAX - 1, 1), u64::MAX);
        assert_eq!(checked_deadline(u64::MAX, 1), None);
        assert_eq!(saturating_deadline(u64::MAX, 1), u64::MAX);
        assert_eq!(checked_deadline(u64::MAX, 0), Some(u64::MAX));
    }
}
