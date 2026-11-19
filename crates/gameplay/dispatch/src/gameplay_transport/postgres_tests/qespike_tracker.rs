//! Crystal 201189f8 actions_ghost_detector.lua:74-103 and gnomilly.lua:119-121,211-237.
use super::TestResult;
use super::qespike_tracker_fixtures::{FINAL_NAME, NAME, REPEAT_NAME, discovery_edge};

#[test]
fn spike_detector_reuses_personal_target_and_advances_without_spending_first_use() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, discovery_edge(NAME)?, false)
    })
}

#[test]
fn spike_detector_resamples_after_gnomilly_records_a_new_daily_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(REPEAT_NAME, || {
        super::quest_world_callbacks_fixtures::scenario(
            REPEAT_NAME,
            discovery_edge(REPEAT_NAME)?,
            false,
        )
    })
}

#[test]
fn spike_detector_final_floor_hotkey_advances_progress_and_consumes_the_detector() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(FINAL_NAME, || {
        super::quest_world_callbacks_fixtures::scenario(
            FINAL_NAME,
            discovery_edge(FINAL_NAME)?,
            false,
        )
    })
}
