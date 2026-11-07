//! CLOCK-1 arithmetic shared by World presentation and durable quest predicates.
/// Real milliseconds per Tibian minute (WORLDINT0-RL-16).
pub const TIBIAN_MINUTE_MS: u64 = 2_500;
/// Tibian minutes in one complete day.
pub const TIBIAN_MINUTES_PER_DAY: u64 = 1_440;
/// Resolve the minute of a recorded server UTC fact; fail before the retained epoch.
pub fn tibian_minute(epoch_utc_ms: u64, recorded_utc_ms: u64) -> Option<u32> {
    let elapsed = recorded_utc_ms.checked_sub(epoch_utc_ms)?;
    u32::try_from((elapsed / TIBIAN_MINUTE_MS) % TIBIAN_MINUTES_PER_DAY).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn minute_edges_epoch_and_full_days_are_stable() {
        assert_eq!(tibian_minute(0, 0), Some(0));
        assert_eq!(tibian_minute(0, 2_499), Some(0));
        assert_eq!(tibian_minute(0, 2_500), Some(1));
        assert_eq!(tibian_minute(0, 3_600_000), Some(0));
        assert_eq!(tibian_minute(3_600_000, 3_599_999), None);
        assert_eq!(tibian_minute(3_600_000, 3_602_500), Some(1));
    }
}
