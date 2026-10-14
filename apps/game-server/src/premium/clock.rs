//! PREM-WIRE-1: the system trusted clock (PREMIUM-ACTIVATION-0 §1.1, PREMIUM-DELIVERY-0 §5).
//!
//! The kernel's NTP-disciplined clock, read by `oteryn-ntp-clock` (owner decision 2026-10-14,
//! PREMIUM-1 #2110, isolated FFI). One read returns the clock and its maximum error together:
//! `now` is the clock, `uncertainty` is the maximum error, and [`TrustedNow::new`] applies the
//! 5 s bound unchanged. Fail closed: an unsynchronized kernel clock, a failed read and a target
//! other than Linux read `None`, which means Premium is not current. Nothing is cached.

use super::TrustedNow;

/// The kernel's synchronized time now, or `None` while it is not trustworthy.
#[must_use]
pub fn system_trusted_now() -> Option<TrustedNow> {
    let clock = oteryn_ntp_clock::read_synchronized()?;
    TrustedNow::new(clock.now_us, clock.max_error_us)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_reading_is_none_or_within_the_uncertainty_bound() {
        if let Some(now) = system_trusted_now() {
            assert!(now.upper_us() - now.lower_us() <= 10_000_000);
        }
    }
}
