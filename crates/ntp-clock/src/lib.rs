//! The kernel's NTP-disciplined realtime clock and its error bound, read once.
//!
//! Owner decision 2026-10-14, PREMIUM-1 #2110, isolated FFI: the one `adjtimex(2)` read
//! (`ntp_adjtime(3)` is the same read under its glibc name) is the only `unsafe` code of the
//! workspace, kept in this crate so every other crate keeps `unsafe_code = "forbid"`.
//!
//! The read is read only (`modes = 0`). `chronyd`, `ntpd` and `systemd-timesyncd` keep the state
//! it returns current; this crate writes no time and runs no NTP client. One call returns the
//! clock and its maximum error together. Fail closed: the clock state `TIME_ERROR`, the status
//! `STA_UNSYNC`, a failed call, an unrepresentable reading and a target other than Linux all read
//! `None`. Nothing is cached.

/// A synchronized kernel reading: the realtime clock and its maximum error, in microseconds
/// (Unix time).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SynchronizedClock {
    pub now_us: i64,
    pub max_error_us: i64,
}

/// The kernel clock now, or `None` while the kernel does not report it synchronized.
#[must_use]
pub fn read_synchronized() -> Option<SynchronizedClock> {
    imp::read()
}

/// One kernel reading: the returned clock state, the status bits, the realtime clock (seconds and
/// the sub-second part, in nanoseconds under `STA_NANO` and microseconds otherwise) and the
/// maximum error in microseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
struct KernelReading {
    state: i32,
    status: i32,
    seconds: i64,
    fraction: i64,
    max_error_us: i64,
}

/// `adjtimex` clock state: the clock is not synchronized.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const TIME_ERROR: i32 = 5;
/// `adjtimex` status: the clock is unsynchronized.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const STA_UNSYNC: i32 = 0x0040;
/// `adjtimex` status: `time` carries nanoseconds.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const STA_NANO: i32 = 0x2000;

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn synchronized(reading: KernelReading) -> Option<SynchronizedClock> {
    if reading.state < 0 || reading.state == TIME_ERROR || reading.status & STA_UNSYNC != 0 {
        return None;
    }
    let fraction_us = if reading.status & STA_NANO != 0 {
        reading.fraction / 1_000
    } else {
        reading.fraction
    };
    if !(0..1_000_000).contains(&fraction_us) || reading.max_error_us < 0 {
        return None;
    }
    let now_us = reading
        .seconds
        .checked_mul(1_000_000)?
        .checked_add(fraction_us)?;
    Some(SynchronizedClock {
        now_us,
        max_error_us: reading.max_error_us,
    })
}

#[cfg(target_os = "linux")]
mod imp {
    use super::{KernelReading, SynchronizedClock, synchronized};

    #[allow(unsafe_code)]
    pub(super) fn read() -> Option<SynchronizedClock> {
        // SAFETY: `timex` is a plain C struct of integers for which all-zero bytes are a valid
        // value; zero `modes` makes the call read only.
        let mut timex: libc::timex = unsafe { std::mem::zeroed() };
        // SAFETY: `timex` is a valid, exclusively borrowed `struct timex` for the whole call.
        let state = unsafe { libc::adjtimex(&mut timex) };
        synchronized(KernelReading {
            state,
            status: timex.status,
            seconds: timex.time.tv_sec,
            fraction: timex.time.tv_usec,
            max_error_us: timex.maxerror,
        })
    }
}

#[cfg(not(target_os = "linux"))]
mod imp {
    use super::SynchronizedClock;

    pub(super) const fn read() -> Option<SynchronizedClock> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-01T00:00:00Z.
    const SECONDS: i64 = 1_790_812_800;
    const SYNCED: KernelReading = KernelReading {
        state: 0,
        status: 0,
        seconds: SECONDS,
        fraction: 250_000,
        max_error_us: 1_000,
    };
    const NOW: SynchronizedClock = SynchronizedClock {
        now_us: SECONDS * 1_000_000 + 250_000,
        max_error_us: 1_000,
    };

    #[test]
    fn a_synchronized_reading_is_the_kernel_time_and_its_error_bound() {
        assert_eq!(synchronized(SYNCED), Some(NOW));
        let nano = KernelReading {
            status: STA_NANO,
            fraction: 250_000_999,
            ..SYNCED
        };
        assert_eq!(synchronized(nano), Some(NOW));
    }

    #[test]
    fn an_unsynchronized_failed_or_malformed_reading_is_none() {
        for unsynchronized in [
            KernelReading {
                state: TIME_ERROR,
                ..SYNCED
            },
            KernelReading {
                status: STA_UNSYNC,
                ..SYNCED
            },
            KernelReading {
                state: -1,
                ..SYNCED
            },
            KernelReading {
                seconds: i64::MAX,
                ..SYNCED
            },
            KernelReading {
                fraction: 1_000_000,
                ..SYNCED
            },
            KernelReading {
                max_error_us: -1,
                ..SYNCED
            },
        ] {
            assert_eq!(synchronized(unsynchronized), None, "{unsynchronized:?}");
        }
    }

    #[test]
    fn the_system_reading_never_panics() {
        let _ = read_synchronized();
    }
}
