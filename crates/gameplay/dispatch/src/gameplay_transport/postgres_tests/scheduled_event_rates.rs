//! W05d: a scheduled rate event reaches the production kill settle (experience and loot) only
//! inside its window; both edges are inclusive.
use super::TestResult;
use oteryn_gameplay_foundation::scheduled_events::{Schedule, test_clock};

/// 2030-01-01T00:00:00Z and 2030-01-02T23:59:59Z.
const START: i64 = 1_893_456_000;
const END: i64 = 1_893_628_799;

fn scenario_at(now: i64) -> TestResult {
    let schedule = Schedule::parse(
        r#"{"events":[{"name":"w05d","startdate":"01/01/2030","enddate":"01/02/2030",
            "ingame":{"exprate":300,"lootrate":25}}]}"#,
    )
    .map_err(|error| format!("{error:?}"))?;
    let _clock = test_clock::set(schedule, now);
    super::kill_reward_e2e::scenario()
}

#[test]
fn w05d_event_scales_real_kill_xp_and_loot_only_inside_its_window_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("w05d-event-rates", || {
        for now in [START - 1, START, END, END + 1] {
            scenario_at(now)?;
        }
        eprintln!("W05D PostgreSQL event-rate kill settle verified at 4 window instants");
        Ok(())
    })
}
