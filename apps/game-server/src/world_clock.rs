//! LOOP-CLOCK-1: the World's Tibian clock and light (WORLD-INTERACTION-0 §11).
//!
//! One clock per World: the Tibian minute is a pure function of a UTC time fact and the World's
//! `world_clock_epoch_utc_ms`, so every channel of a World agrees without sending the clock to
//! another. The caller supplies the time fact (the input's or tick's recorded UTC time,
//! SIM-DETERMINISM-01 §15); only `now_utc_ms` reads the wall clock, once per process. The wire
//! payload is `oteryn_protocol_oteryn::world_clock`.

use oteryn_protocol_oteryn::world_clock::{TIBIAN_MINUTES_PER_DAY, WorldClock};

/// `WORLDINT0-RL-16`: real milliseconds per Tibian minute.
pub(crate) const TIBIAN_MINUTE_MS: u64 = 2_500;
/// `WORLDINT0-RL-17`: the light level at night and by day, and the light color.
pub(crate) const NIGHT_LIGHT_LEVEL: u8 = 40;
pub(crate) const DAY_LIGHT_LEVEL: u8 = 250;
pub(crate) const LIGHT_COLOR: u8 = 215;
/// `WORLDINT0-RL-17`: the light step of 7 per 4 Tibian minutes, and the sunrise (06:00) and
/// sunset (17:30) starts, in Tibian minutes.
const LIGHT_STEP: u32 = 7;
const LIGHT_STEP_MINUTES: u32 = 4;
const SUNRISE_START: u32 = 360;
const DAY_START: u32 = 480;
const SUNSET_START: u32 = 1_050;
const NIGHT_START: u32 = 1_170;
/// `WORLDINT0-RL-18`: at most one `WORLD_CLOCK` delta per session in this many milliseconds.
pub(crate) const MIN_DELTA_INTERVAL_MS: u64 = 10_000;

/// CLOCK-1: the World clock epoch while no World ruleset carries `world_clock_epoch_utc_ms`: the
/// Unix epoch, a whole UTC hour (§11.1).
pub(crate) const DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS: u64 = 0;

/// The process's UTC time fact in milliseconds (§11.2), normalized onto the monotonic owner clock:
/// the wall clock is read once, on the first call, and every later fact is that reading plus the
/// monotonic time since it. A wall-clock adjustment therefore never moves the World clock, and
/// the facts never go backward. `None` when the wall clock cannot be read.
pub(crate) fn now_utc_ms() -> Option<u64> {
    static ANCHOR: std::sync::OnceLock<Option<(u64, std::time::Instant)>> =
        std::sync::OnceLock::new();
    let (utc_ms, at) = (*ANCHOR.get_or_init(|| {
        let elapsed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?;
        Some((
            u64::try_from(elapsed.as_millis()).ok()?,
            std::time::Instant::now(),
        ))
    }))?;
    utc_ms.checked_add(u64::try_from(at.elapsed().as_millis()).ok()?)
}
const TIBIAN_HOUR_MINUTES: u32 = 60;

/// §11.3: the World light level at a Tibian minute of the day. Panics never; a minute outside the
/// day is reduced modulo the day.
pub(crate) fn light_level(minute: u32) -> u8 {
    let minute = minute % TIBIAN_MINUTES_PER_DAY;
    let (night, day) = (u32::from(NIGHT_LIGHT_LEVEL), u32::from(DAY_LIGHT_LEVEL));
    let level = match minute {
        SUNRISE_START..DAY_START => {
            (night + LIGHT_STEP * ((minute - SUNRISE_START) / LIGHT_STEP_MINUTES + 1)).min(day)
        }
        DAY_START..SUNSET_START => day,
        SUNSET_START..NIGHT_START => day
            .saturating_sub(LIGHT_STEP * ((minute - SUNSET_START) / LIGHT_STEP_MINUTES + 1))
            .max(night),
        _ => night,
    };
    // Both bounds are u8 values, so the level always fits.
    u8::try_from(level).unwrap_or(DAY_LIGHT_LEVEL)
}

/// §11.5: `is_day`, 06:00 to 18:00.
pub(crate) fn is_day(minute: u32) -> bool {
    (SUNRISE_START..SUNSET_START + 30).contains(&(minute % TIBIAN_MINUTES_PER_DAY))
}

/// §11.5: the `world_time {from_minute, to_minute}` predicate on a Tibian minute; the range is
/// inclusive of `from`, exclusive of `to`, and wraps over midnight when `from > to`. An empty
/// range (`from == to`) never holds.
pub(crate) fn in_world_time(minute: u32, from_minute: u32, to_minute: u32) -> bool {
    let minute = minute % TIBIAN_MINUTES_PER_DAY;
    match from_minute.cmp(&to_minute) {
        std::cmp::Ordering::Less => (from_minute..to_minute).contains(&minute),
        std::cmp::Ordering::Greater => minute >= from_minute || minute < to_minute,
        std::cmp::Ordering::Equal => false,
    }
}

/// The World's clock: its epoch only. The default epoch is a whole UTC hour, so Tibian midnight
/// falls on each full real hour (§11.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorldClockSource {
    epoch_utc_ms: u64,
}

impl WorldClockSource {
    pub(crate) fn new(epoch_utc_ms: u64) -> Self {
        Self { epoch_utc_ms }
    }

    /// Read the absolute elapsed time from this clock's retained epoch. Unlike the Tibian
    /// minute, this value does not wrap at midnight. The caller supplies the recorded UTC
    /// fact; no wall clock or fallback epoch is consulted here.
    ///
    /// This arithmetic read does not prove World-config provenance. In particular, the
    /// legacy epoch-zero clock at the transport caller is not a weather binding (D1025).
    pub(crate) fn absolute_elapsed_ms(&self, recorded_utc_ms: u64) -> Option<u64> {
        recorded_utc_ms.checked_sub(self.epoch_utc_ms)
    }

    /// §11.2: `floor((t - epoch) / 2,500 ms) mod 1,440`. `None` before the epoch.
    pub(crate) fn tibian_minute(&self, now_utc_ms: u64) -> Option<u32> {
        let elapsed = self.absolute_elapsed_ms(now_utc_ms)?;
        u32::try_from((elapsed / TIBIAN_MINUTE_MS) % u64::from(TIBIAN_MINUTES_PER_DAY)).ok()
    }

    /// The wire payload at a time fact; `None` before the epoch.
    pub(crate) fn reading(&self, now_utc_ms: u64) -> Option<WorldClock> {
        let minute = self.tibian_minute(now_utc_ms)?;
        Some(WorldClock {
            tibian_minute: u16::try_from(minute).ok()?,
            light_level: light_level(minute),
            light_color: LIGHT_COLOR,
        })
    }
}

/// Decides when a session's `WORLD_CLOCK` delta is due: when the light level changes and at each
/// Tibian hour, at most one per `MIN_DELTA_INTERVAL_MS`. The snapshot at admission, reconnect and
/// channel transfer is `WorldClockSource::reading` and calls `reset`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct WorldClockDeltaPacer {
    last: Option<(WorldClock, u64)>,
}

impl WorldClockDeltaPacer {
    /// Records the snapshot just sent at `now_utc_ms`.
    pub(crate) fn reset(&mut self, snapshot: WorldClock, now_utc_ms: u64) {
        self.last = Some((snapshot, now_utc_ms));
    }

    /// The delta to send at `now_utc_ms`, if any. Nothing is due before a snapshot was recorded.
    pub(crate) fn due(&mut self, source: &WorldClockSource, now_utc_ms: u64) -> Option<WorldClock> {
        let (previous, sent_at) = self.last?;
        let current = source.reading(now_utc_ms)?;
        let elapsed = now_utc_ms.checked_sub(sent_at)?;
        let new_hour = u32::from(current.tibian_minute) / TIBIAN_HOUR_MINUTES
            != u32::from(previous.tibian_minute) / TIBIAN_HOUR_MINUTES
            || elapsed >= u64::from(TIBIAN_MINUTES_PER_DAY) * TIBIAN_MINUTE_MS;
        let changed = current.light_level != previous.light_level || new_hour;
        if !changed || elapsed < MIN_DELTA_INTERVAL_MS {
            return None;
        }
        self.last = Some((current, now_utc_ms));
        Some(current)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    const HOUR_MS: u64 = 3_600_000;

    #[test]
    fn the_utc_fact_is_anchored_once_and_never_goes_backward() {
        let first = now_utc_ms().expect("utc fact");
        let second = now_utc_ms().expect("utc fact");
        assert!(second >= first);
        assert!(first > 0);
    }

    #[test]
    fn minute_is_a_pure_function_of_the_epoch() {
        let source = WorldClockSource::new(5 * HOUR_MS);
        assert_eq!(source.absolute_elapsed_ms(5 * HOUR_MS - 1), None);
        assert_eq!(source.absolute_elapsed_ms(5 * HOUR_MS), Some(0));
        assert_eq!(source.absolute_elapsed_ms(6 * HOUR_MS), Some(HOUR_MS));
        assert_eq!(
            source.absolute_elapsed_ms(u64::MAX),
            Some(u64::MAX - 5 * HOUR_MS)
        );
        assert_eq!(source.tibian_minute(5 * HOUR_MS - 1), None);
        assert_eq!(source.tibian_minute(5 * HOUR_MS), Some(0));
        assert_eq!(source.tibian_minute(5 * HOUR_MS + 2_499), Some(0));
        assert_eq!(source.tibian_minute(5 * HOUR_MS + 2_500), Some(1));
        // 150 s per Tibian hour; one Tibian day per real hour, wrapping to midnight.
        assert_eq!(source.tibian_minute(5 * HOUR_MS + 150_000), Some(60));
        assert_eq!(source.tibian_minute(6 * HOUR_MS - 1), Some(1_439));
        assert_eq!(source.tibian_minute(6 * HOUR_MS), Some(0));
        assert!(source.tibian_minute(u64::MAX).is_some());
    }

    #[test]
    fn light_follows_the_registered_curve() {
        for (minute, level) in [
            (0, 40),
            (359, 40),
            (360, 47),
            (363, 47),
            (364, 54),
            (479, 250),
            (480, 250),
            (1_049, 250),
            (1_050, 243),
            (1_053, 243),
            (1_054, 236),
            (1_169, 40),
            (1_170, 40),
            (1_439, 40),
        ] {
            assert_eq!(light_level(minute), level, "minute {minute}");
        }
        // Sunrise reaches the day level without overshoot and never drops below night.
        assert!((0..1_440).all(|m| (40..=250).contains(&light_level(m))));
        assert_eq!(light_level(360 + 4 * 28), 243);
        assert_eq!(light_level(360 + 4 * 29), 250);
    }

    #[test]
    fn day_and_world_time_predicates() {
        assert!(!is_day(359) && is_day(360) && is_day(1_079) && !is_day(1_080));
        assert!(in_world_time(100, 60, 200));
        assert!(!in_world_time(200, 60, 200));
        // Wraps over midnight.
        assert!(in_world_time(1_430, 1_380, 60) && in_world_time(30, 1_380, 60));
        assert!(!in_world_time(600, 1_380, 60));
        assert!(!in_world_time(5, 7, 7));
    }

    #[test]
    fn reading_carries_minute_light_and_color() {
        let source = WorldClockSource::new(0);
        let reading = source.reading(480 * TIBIAN_MINUTE_MS).expect("reading");
        assert_eq!(
            reading,
            WorldClock {
                tibian_minute: 480,
                light_level: 250,
                light_color: 215
            }
        );
        assert_eq!(WorldClockSource::new(HOUR_MS).reading(0), None);
    }

    #[test]
    fn deltas_follow_light_changes_and_hours_at_most_every_ten_seconds() {
        let source = WorldClockSource::new(0);
        let mut pacer = WorldClockDeltaPacer::default();
        // Nothing is due before a snapshot.
        assert_eq!(pacer.due(&source, 0), None);
        let at = |minute: u64| minute * TIBIAN_MINUTE_MS;
        pacer.reset(source.reading(at(100)).expect("reading"), at(100));
        // Night is flat inside an hour: no change, nothing due.
        assert_eq!(pacer.due(&source, at(110)), None);
        // A new Tibian hour (minute 120) is due although the level is unchanged.
        let hour = pacer.due(&source, at(120)).expect("hour");
        assert_eq!(hour.tibian_minute, 120);
        assert_eq!(pacer.due(&source, at(121)), None);

        // Sunrise changes the level every 4 minutes (10 s); the cap never delays that step.
        pacer.reset(source.reading(at(360)).expect("reading"), at(360));
        assert_eq!(pacer.due(&source, at(363)), None);
        assert_eq!(pacer.due(&source, at(364)).expect("step").light_level, 54);
        // The same reading is not due twice, and a hand-back inside the cap is held.
        assert_eq!(pacer.due(&source, at(364)), None);
        pacer.reset(source.reading(at(364)).expect("reading"), at(364));
        assert_eq!(pacer.due(&source, at(364) + 9_999), None);
        assert!(pacer.due(&source, at(368)).is_some());
    }
}
