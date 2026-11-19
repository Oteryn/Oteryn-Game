//! Crystal 201189f8 data-global/scripts/quests/spike_tasks/actions_thermometer.lua:1-36.
use super::TestResult;
use super::qespike_hotspot_fixtures::{NAME, selected_pool};

#[test]
fn spike_thermometer_hotkey_only_consumes_at_the_shared_server_save_hotspot() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, selected_pool()?, false)
    })
}
