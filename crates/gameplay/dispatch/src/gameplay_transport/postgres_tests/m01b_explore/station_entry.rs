use super::*;

#[test]
fn m01b_horse_station_native_landmark_visit_never_activates_escape_state() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-station-visit", || {
        super::super::quest_world_callbacks_fixtures::scenario(
            "m01b/horse-station-world-change/s1/entry",
            Position {
                x: 32442,
                y: 32229,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_station_visit duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
