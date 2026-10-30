//! W05D's bounded, content-owned event-rate schedule.

use serde::Deserialize;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEDULE: &str = include_str!("../../../../content/events/schedule.json");
// Crystal events_scheduler.cpp:68-75 resets every schedule to 100; its XML loader reads the four
// rates at lines 369-396 and the setters compound percentages (events_scheduler.hpp:64-110).
const BASE_RATE: u32 = 100;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct EventRates {
    pub experience: u32,
    pub loot: u32,
    pub spawn: u32,
    pub skill: u32,
}

impl Default for EventRates {
    fn default() -> Self {
        Self {
            experience: BASE_RATE,
            loot: BASE_RATE,
            spawn: BASE_RATE,
            skill: BASE_RATE,
        }
    }
}

#[derive(Debug, Deserialize)]
struct Schedule {
    events: Vec<ScheduledEvent>,
}

#[derive(Debug, Deserialize)]
struct ScheduledEvent {
    start_unix: u64,
    end_unix: u64,
    rates: EventRates,
}

fn schedule() -> &'static Schedule {
    static EVENTS: OnceLock<Schedule> = OnceLock::new();
    EVENTS.get_or_init(|| {
        serde_json::from_str(SCHEDULE).unwrap_or_else(|_| Schedule { events: Vec::new() })
    })
}

/// Rates at an inclusive schedule instant. Overlapping events compound, as Crystal's setters do.
pub fn rates_at(unix_seconds: u64) -> EventRates {
    schedule()
        .events
        .iter()
        .filter(|event| unix_seconds >= event.start_unix && unix_seconds <= event.end_unix)
        .fold(EventRates::default(), |mut active, event| {
            active.experience =
                active.experience.saturating_mul(event.rates.experience) / BASE_RATE;
            active.loot = active.loot.saturating_mul(event.rates.loot) / BASE_RATE;
            active.spawn = active.spawn.saturating_mul(event.rates.spawn) / BASE_RATE;
            active.skill = active.skill.saturating_mul(event.rates.skill) / BASE_RATE;
            active
        })
}

pub fn current_rates() -> EventRates {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    rates_at(now)
}

pub const fn apply_gain(value: u64, percent: u32) -> u64 {
    value.saturating_mul(percent as u64) / BASE_RATE as u64
}

pub const fn apply_signed_gain(value: i64, percent: u32) -> i64 {
    value.saturating_mul(percent as i64) / BASE_RATE as i64
}

pub const fn apply_spawn_delay(value: u64, percent: u32) -> u64 {
    let divisor = if percent == 0 { 1 } else { percent };
    value
        .saturating_mul(BASE_RATE as u64)
        .saturating_div(divisor as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_edges_are_inclusive_and_outside_is_neutral() {
        let before = rates_at(1_893_455_999);
        let start = rates_at(1_893_456_000);
        let end = rates_at(1_893_542_399);
        let after = rates_at(1_893_542_400);
        assert_eq!(before, EventRates::default());
        assert_eq!(
            start,
            EventRates {
                experience: 200,
                loot: 200,
                spawn: 200,
                skill: 200
            }
        );
        assert_eq!(end, start);
        assert_eq!(after, EventRates::default());
        eprintln!("W05D_EVENT_EDGES_EXECUTED before/start/end/after=4");
    }
}
