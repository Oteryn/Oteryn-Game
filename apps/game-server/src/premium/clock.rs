//! PREM-WIRE-1: the system trusted clock (PREMIUM-ACTIVATION-0 §1.1, PREMIUM-DELIVERY-0 §5).
//!
//! The kernel's NTP-disciplined clock, read by `oteryn-ntp-clock` (owner decision 2026-10-14,
//! PREMIUM-1 #2110, isolated FFI). One read returns the clock and its maximum error together:
//! `now` is the clock, `uncertainty` is the maximum error, and [`TrustedNow::new`] applies the
//! 5 s bound unchanged. Fail closed: an unsynchronized kernel clock, a failed read and a target
//! other than Linux read `None`, which means Premium is not current.
//!
//! Rollback: the process keeps one high-water mark, the greatest lower bound it has trusted. Real
//! time never goes back, so a reading whose upper bound is below that mark means the clock was
//! stepped backward; it reads `None` until the clock catches up, so a backward step can never
//! keep a grant current past its real cutoff. No reading itself is cached.

use std::sync::atomic::{AtomicI64, Ordering};

use super::TrustedNow;

/// The greatest trusted lower bound this process has returned, in Unix microseconds.
static HIGH_WATER_US: AtomicI64 = AtomicI64::new(i64::MIN);

/// The kernel's synchronized time now, or `None` while it is not trustworthy.
#[must_use]
pub fn system_trusted_now() -> Option<TrustedNow> {
    let clock = oteryn_ntp_clock::read_synchronized()?;
    admit_monotonic(
        &HIGH_WATER_US,
        TrustedNow::new(clock.now_us, clock.max_error_us)?,
    )
}

/// `None` when `reading` lies wholly before a lower bound already trusted (a clock rollback);
/// otherwise raises the mark to the reading's lower bound and returns it.
fn admit_monotonic(high_water_us: &AtomicI64, reading: TrustedNow) -> Option<TrustedNow> {
    if reading.upper_us() < high_water_us.load(Ordering::Acquire) {
        return None;
    }
    high_water_us.fetch_max(reading.lower_us(), Ordering::AcqRel);
    Some(reading)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn the_system_reading_is_none_or_within_the_uncertainty_bound() {
        if let Some(now) = system_trusted_now() {
            assert!(now.upper_us() - now.lower_us() <= 10_000_000);
        }
    }

    #[test]
    fn a_reading_before_the_trusted_high_water_mark_fails_closed() {
        let mark = AtomicI64::new(i64::MIN);
        let at = |now_us| TrustedNow::new(now_us, 1_000_000).unwrap();
        assert_eq!(
            admit_monotonic(&mark, at(100_000_000)),
            Some(at(100_000_000))
        );
        assert_eq!(mark.load(Ordering::Acquire), 99_000_000);
        // Stepped back 60 s: wholly before the trusted lower bound.
        assert_eq!(admit_monotonic(&mark, at(40_000_000)), None);
        assert_eq!(mark.load(Ordering::Acquire), 99_000_000);
        // A step back within the uncertainty overlaps the mark and stays admitted.
        assert_eq!(admit_monotonic(&mark, at(99_500_000)), Some(at(99_500_000)));
        assert_eq!(mark.load(Ordering::Acquire), 99_000_000);
        // Once the clock catches up, readings are admitted and the mark advances.
        assert_eq!(
            admit_monotonic(&mark, at(101_000_000)),
            Some(at(101_000_000))
        );
        assert_eq!(mark.load(Ordering::Acquire), 100_000_000);
    }
}
