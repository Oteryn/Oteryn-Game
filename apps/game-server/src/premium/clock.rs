//! PREM-WIRE-1: the system trusted clock (PREMIUM-ACTIVATION-0 §1.1, PREMIUM-DELIVERY-0 §5).
//!
//! The kernel's NTP-disciplined clock, read by `oteryn-ntp-clock` (owner decision 2026-10-14,
//! PREMIUM-1 #2110, isolated FFI). One read returns the clock and its maximum error together:
//! `now` is the clock, `uncertainty` is the maximum error, and [`TrustedNow::new`] applies the
//! 5 s bound unchanged. Fail closed: an unsynchronized kernel clock, a failed read and a target
//! other than Linux read `None`, which means Premium is not current.
//!
//! True time never runs backward, so the process keeps the highest bounds it has read. A later
//! reading keeps its own lower bound, and its upper bound is raised to the highest upper bound
//! already read, so a step back of the kernel clock never makes Premium current again after a
//! cutoff the process has passed. A reading wholly before an earlier lower bound contradicts it
//! and reads `None`.

use super::TrustedNow;
use std::sync::Mutex;

/// The highest lower and upper bounds this process has read.
static HIGH_WATER: Mutex<Option<TrustedNow>> = Mutex::new(None);

/// The kernel's synchronized time now, or `None` while it is not trustworthy.
#[must_use]
pub fn system_trusted_now() -> Option<TrustedNow> {
    let clock = oteryn_ntp_clock::read_synchronized()?;
    let reading = TrustedNow::new(clock.now_us, clock.max_error_us)?;
    let mut high_water = HIGH_WATER.lock().ok()?;
    hold_monotonic(&mut high_water, reading)
}

/// `reading` held to the bounds already read; `None` when it lies wholly before them.
fn hold_monotonic(high_water: &mut Option<TrustedNow>, reading: TrustedNow) -> Option<TrustedNow> {
    let Some(seen) = *high_water else {
        *high_water = Some(reading);
        return Some(reading);
    };
    if reading.upper_us < seen.lower_us {
        return None;
    }
    *high_water = Some(TrustedNow {
        lower_us: seen.lower_us.max(reading.lower_us),
        upper_us: seen.upper_us.max(reading.upper_us),
    });
    Some(TrustedNow {
        lower_us: reading.lower_us,
        upper_us: reading.upper_us.max(seen.upper_us),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(now_us: i64, uncertainty_us: i64) -> TrustedNow {
        TrustedNow {
            lower_us: now_us - uncertainty_us,
            upper_us: now_us + uncertainty_us,
        }
    }

    fn bounds(now: Option<TrustedNow>) -> Option<(i64, i64)> {
        now.map(|now| (now.lower_us(), now.upper_us()))
    }

    #[test]
    fn the_system_reading_is_none_or_within_the_uncertainty_bound() {
        if let Some(now) = system_trusted_now() {
            assert!(now.upper_us() - now.lower_us() <= 10_000_000);
        }
    }

    #[test]
    fn a_forward_reading_passes_unchanged() {
        let mut high_water = None;
        assert_eq!(
            hold_monotonic(&mut high_water, at(1_000, 10)),
            Some(at(1_000, 10))
        );
        assert_eq!(
            hold_monotonic(&mut high_water, at(2_000, 10)),
            Some(at(2_000, 10))
        );
    }

    #[test]
    fn a_step_back_keeps_the_upper_bound_already_read() {
        let mut high_water = None;
        hold_monotonic(&mut high_water, at(2_000, 100));
        // Back by 150 us: inside the earlier window, so consistent, but the cutoff stays passed.
        assert_eq!(
            bounds(hold_monotonic(&mut high_water, at(1_850, 100))),
            Some((1_750, 2_100))
        );
        // A smaller maximum error does not lower the upper bound either.
        assert_eq!(
            bounds(hold_monotonic(&mut high_water, at(2_010, 1))),
            Some((2_009, 2_100))
        );
    }

    #[test]
    fn a_reading_wholly_before_an_earlier_lower_bound_reads_none() {
        let mut high_water = None;
        hold_monotonic(&mut high_water, at(1_000_000, 10));
        assert_eq!(hold_monotonic(&mut high_water, at(500_000, 10)), None);
        // The high water is kept: the clock is trusted again once it passes it.
        assert_eq!(
            bounds(hold_monotonic(&mut high_water, at(999_995, 10))),
            Some((999_985, 1_000_010))
        );
    }
}
