//! W05d events scheduler: rate events active between a start and an end instant.
//!
//! Donor: zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8 (an OTS hypothesis
//! source), `data/XML/events.xml` read by `src/game/scheduling/events_scheduler.cpp`.
//! - Window: `startdate`/`enddate` are `MM/DD/YYYY` (`events_scheduler.cpp:29-33`); `hour`
//!   sets both times, `starthour` defaults to `00:00` and `endhour` to `23:59:59` (`:336-338`);
//!   an end before its start is refused (`:345-348`); an event is active while
//!   `start <= now <= end`, both edges inclusive (`:350-352`).
//! - Rates: whole percentages, 100 neutral (`data/global.lua:69-73`); each present `ingame`
//!   attribute compounds `schedule * rate / 100` (`events_scheduler.cpp:369-397`, setters
//!   `events_scheduler.hpp:67-111`), so overlapping events multiply.
//! - Uses: experience `data/libs/functions/player.lua:428-429`; loot chance
//!   `data/libs/functions/functions.lua:68-70`; respawn interval `* 100 / rate`, kept within
//!   1 s..1 day (`src/creatures/monsters/spawns/spawn_monster.cpp:443-458`); skill and magic
//!   tries `data/events/scripts/player.lua:658`.
//!
//! Crystal reads the host's local clock once at startup. Oteryn reads UTC at each use, as the
//! Rashid rotation fixes its calendar to UTC (`rashid_schedule.rs`).

use serde::Deserialize;
use std::sync::OnceLock;

const SCHEDULE: &str = include_str!("../../../../content/encounters/event-schedule/schedule.json");
/// `data/global.lua:69-73`: the neutral rate of every schedule.
pub const NEUTRAL_RATE: u32 = 100;
/// `spawn_monster.cpp:35-36`: an event-scaled respawn interval stays within 1 s and one day.
const MIN_RESPAWN_MICROS: u64 = 1_000_000;
const MAX_RESPAWN_MICROS: u64 = 86_400_000_000;

/// The four admitted multipliers, whole percentages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventRates {
    pub experience: u32,
    pub loot: u32,
    pub spawn: u32,
    pub skill: u32,
}

impl Default for EventRates {
    fn default() -> Self {
        Self {
            experience: NEUTRAL_RATE,
            loot: NEUTRAL_RATE,
            spawn: NEUTRAL_RATE,
            skill: NEUTRAL_RATE,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceSchedule {
    events: Vec<SourceEvent>,
}

/// One `events.xml` `<event>` with its `<ingame>` rates (absent attributes stay 100).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceEvent {
    name: String,
    startdate: String,
    enddate: String,
    #[serde(default)]
    hour: Option<String>,
    #[serde(default)]
    starthour: Option<String>,
    #[serde(default)]
    endhour: Option<String>,
    #[serde(default)]
    ingame: SourceRates,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceRates {
    exprate: Option<u32>,
    lootrate: Option<u32>,
    spawnrate: Option<u32>,
    skillrate: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScheduledEvent {
    start_unix: i64,
    end_unix: i64,
    rates: EventRates,
}

/// Why a schedule source was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    Json(String),
    /// The event's date or time is not a valid `MM/DD/YYYY` / `HH:MM[:SS]`.
    DateTime(String),
    /// The event ends before it starts.
    EndBeforeStart(String),
}

/// A validated event schedule.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schedule {
    events: Vec<ScheduledEvent>,
}

impl Schedule {
    /// Parses the content form of `events.xml`. Unlike Crystal, which logs and skips a bad event
    /// (`events_scheduler.cpp:340-348`), a bad event refuses the whole schedule.
    pub fn parse(source: &str) -> Result<Self, ScheduleError> {
        let source: SourceSchedule =
            serde_json::from_str(source).map_err(|error| ScheduleError::Json(error.to_string()))?;
        let mut events = Vec::with_capacity(source.events.len());
        for event in source.events {
            let start_hour = event
                .starthour
                .as_deref()
                .or(event.hour.as_deref())
                .unwrap_or("00:00");
            let end_hour = event
                .endhour
                .as_deref()
                .or(event.hour.as_deref())
                .unwrap_or("23:59:59");
            let (Some(start_unix), Some(end_unix)) = (
                utc_instant(&event.startdate, start_hour),
                utc_instant(&event.enddate, end_hour),
            ) else {
                return Err(ScheduleError::DateTime(event.name));
            };
            if end_unix < start_unix {
                return Err(ScheduleError::EndBeforeStart(event.name));
            }
            let rate = |rate: Option<u32>| rate.unwrap_or(NEUTRAL_RATE);
            events.push(ScheduledEvent {
                start_unix,
                end_unix,
                rates: EventRates {
                    experience: rate(event.ingame.exprate),
                    loot: rate(event.ingame.lootrate),
                    spawn: rate(event.ingame.spawnrate),
                    skill: rate(event.ingame.skillrate),
                },
            });
        }
        Ok(Self { events })
    }

    /// The compounded rates at a UTC instant; both window edges are inclusive.
    pub fn rates_at(&self, unix_seconds: i64) -> EventRates {
        let compound = |schedule: u32, rate: u32| {
            u32::try_from(u64::from(schedule) * u64::from(rate) / u64::from(NEUTRAL_RATE))
                .unwrap_or(u32::MAX)
        };
        self.events
            .iter()
            .filter(|event| (event.start_unix..=event.end_unix).contains(&unix_seconds))
            .fold(EventRates::default(), |active, event| EventRates {
                experience: compound(active.experience, event.rates.experience),
                loot: compound(active.loot, event.rates.loot),
                spawn: compound(active.spawn, event.rates.spawn),
                skill: compound(active.skill, event.rates.skill),
            })
    }
}

/// The committed schedule under `content/encounters/event-schedule/`. Its parse is pinned by a
/// test; an unreadable schedule would leave every rate neutral.
pub fn content_schedule() -> &'static Schedule {
    static SCHEDULE_CELL: OnceLock<Schedule> = OnceLock::new();
    SCHEDULE_CELL.get_or_init(|| Schedule::parse(SCHEDULE).unwrap_or_default())
}

/// The rates in force now: the content schedule at the current UTC second.
pub fn current_rates() -> EventRates {
    #[cfg(any(test, feature = "test-support"))]
    if let Some(rates) = test_clock::rates() {
        return rates;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .unwrap_or(0);
    content_schedule().rates_at(now)
}

/// Experience, a loot chance or skill tries under `percent` (floor).
pub fn scale_gain(value: u64, percent: u32) -> u64 {
    if percent == NEUTRAL_RATE {
        return value;
    }
    u64::try_from(u128::from(value) * u128::from(percent) / u128::from(NEUTRAL_RATE))
        .unwrap_or(u64::MAX)
}

/// A respawn delay under a spawn rate: `delay * 100 / rate`, kept within 1 s..1 day. A neutral
/// rate leaves the content delay as it is.
pub fn scale_respawn_delay_micros(delay_micros: u64, percent: u32) -> u64 {
    if percent == NEUTRAL_RATE {
        return delay_micros;
    }
    let scaled = u128::from(delay_micros) * u128::from(NEUTRAL_RATE) / u128::from(percent.max(1));
    u64::try_from(scaled)
        .unwrap_or(u64::MAX)
        .clamp(MIN_RESPAWN_MICROS, MAX_RESPAWN_MICROS)
}

/// Seconds since the Unix epoch of a `MM/DD/YYYY` date at `HH:MM[:SS]` UTC
/// (`events_scheduler.cpp:29-63`).
fn utc_instant(date: &str, time: &str) -> Option<i64> {
    let mut date = date.split('/').map(|part| part.trim().parse::<i64>().ok());
    let (Some(Some(month)), Some(Some(day)), Some(Some(year)), None) =
        (date.next(), date.next(), date.next(), date.next())
    else {
        return None;
    };
    let parts: Vec<Option<i64>> = time
        .split(':')
        .map(|part| part.trim().parse::<i64>().ok())
        .collect();
    let (hour, minute, second) = match parts.as_slice() {
        [Some(hour), Some(minute)] => (*hour, *minute, 0),
        [Some(hour), Some(minute), Some(second)] => (*hour, *minute, *second),
        _ => return None,
    };
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if !(1970..=9999).contains(&year)
        || !(1..=month_days).contains(&day)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return None;
    }
    // Howard Hinnant's days-from-civil.
    let shifted = if month <= 2 { year - 1 } else { year };
    let era = shifted.div_euclid(400);
    let year_of_era = shifted - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// A per-thread schedule and clock for tests that drive the real XP, loot, respawn and skill
/// paths across an event window.
#[cfg(any(test, feature = "test-support"))]
pub mod test_clock {
    use super::{EventRates, Schedule};
    use std::cell::RefCell;

    thread_local! {
        static CLOCK: RefCell<Option<(Schedule, i64)>> = const { RefCell::new(None) };
    }

    /// Restores the previous clock of this thread when dropped.
    pub struct Guard(Option<(Schedule, i64)>);

    impl Drop for Guard {
        fn drop(&mut self) {
            let previous = self.0.take();
            CLOCK.with(|clock| *clock.borrow_mut() = previous);
        }
    }

    /// Until the guard drops, `current_rates` on this thread reads `schedule` at `unix_seconds`.
    pub fn set(schedule: Schedule, unix_seconds: i64) -> Guard {
        Guard(CLOCK.with(|clock| clock.borrow_mut().replace((schedule, unix_seconds))))
    }

    pub(super) fn rates() -> Option<EventRates> {
        CLOCK.with(|clock| {
            clock
                .borrow()
                .as_ref()
                .map(|(schedule, now)| schedule.rates_at(*now))
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures")]
mod tests {
    use super::*;

    /// 2030-01-01T00:00:00Z and 2030-01-02T23:59:59Z.
    const START: i64 = 1_893_456_000;
    const END: i64 = 1_893_628_799;

    fn double_weekend() -> Schedule {
        Schedule::parse(
            r#"{"events":[{"name":"double","startdate":"01/01/2030","enddate":"01/02/2030",
                "ingame":{"exprate":200,"lootrate":150,"spawnrate":200,"skillrate":300}}]}"#,
        )
        .expect("schedule")
    }

    #[test]
    fn w05d_window_edges_are_inclusive_and_outside_is_neutral() {
        let schedule = double_weekend();
        let active = EventRates {
            experience: 200,
            loot: 150,
            spawn: 200,
            skill: 300,
        };
        assert_eq!(schedule.rates_at(START - 1), EventRates::default());
        assert_eq!(schedule.rates_at(START), active);
        assert_eq!(schedule.rates_at(END), active);
        assert_eq!(schedule.rates_at(END + 1), EventRates::default());
    }

    #[test]
    fn w05d_hours_overlaps_and_refusals_follow_the_donor_loader() {
        let schedule = Schedule::parse(
            r#"{"events":[
                {"name":"a","startdate":"03/01/2030","enddate":"03/01/2030","starthour":"10:30",
                 "endhour":"12:00","ingame":{"exprate":150}},
                {"name":"b","startdate":"03/01/2030","enddate":"03/01/2030","hour":"11:00",
                 "ingame":{"exprate":200,"lootrate":50}}]}"#,
        )
        .expect("schedule");
        // 2030-03-01T00:00:00Z.
        let day = 1_898_553_600;
        assert_eq!(
            schedule
                .rates_at(day + 10 * 3_600 + 29 * 60 + 59)
                .experience,
            100
        );
        assert_eq!(
            schedule.rates_at(day + 10 * 3_600 + 30 * 60).experience,
            150
        );
        // `hour` makes event b a single instant; both events compound there.
        let both = schedule.rates_at(day + 11 * 3_600);
        assert_eq!((both.experience, both.loot), (300, 50));
        assert_eq!(schedule.rates_at(day + 11 * 3_600 + 1).experience, 150);
        assert_eq!(schedule.rates_at(day + 12 * 3_600 + 1).experience, 100);
        assert_eq!(
            Schedule::parse(
                r#"{"events":[{"name":"x","startdate":"01/02/2030","enddate":"01/01/2030"}]}"#
            ),
            Err(ScheduleError::EndBeforeStart("x".into()))
        );
        assert_eq!(
            Schedule::parse(
                r#"{"events":[{"name":"y","startdate":"02/30/2030","enddate":"03/01/2030"}]}"#
            ),
            Err(ScheduleError::DateTime("y".into()))
        );
    }

    #[test]
    fn w05d_content_schedule_parses_and_is_neutral_now() {
        let schedule = Schedule::parse(SCHEDULE).expect("content schedule");
        assert_eq!(&schedule, content_schedule());
        // The donor's two example events (2024, every rate 100) leave every rate neutral.
        assert_eq!(schedule.events.len(), 2);
        assert_eq!(schedule.rates_at(1_704_067_200), EventRates::default());
        assert_eq!(current_rates(), EventRates::default());
    }

    #[test]
    fn w05d_scaling_matches_the_donor_formulas() {
        assert_eq!(scale_gain(1_000, 200), 2_000);
        assert_eq!(scale_gain(333, 150), 499);
        assert_eq!(scale_gain(7, NEUTRAL_RATE), 7);
        assert_eq!(scale_respawn_delay_micros(60_000_000, 200), 30_000_000);
        assert_eq!(scale_respawn_delay_micros(60_000_000, 50), 120_000_000);
        assert_eq!(scale_respawn_delay_micros(1_500_000, 1_000), 1_000_000);
        assert_eq!(scale_respawn_delay_micros(500_000, NEUTRAL_RATE), 500_000);
        let _clock = test_clock::set(double_weekend(), START);
        assert_eq!(current_rates().spawn, 200);
    }
}
