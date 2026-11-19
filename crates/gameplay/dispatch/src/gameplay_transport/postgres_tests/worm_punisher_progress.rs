use super::TestResult;
use super::worm_punisher_progress_fixtures::{CARRION, HEATING, NAME, ROTATING};

#[test]
fn worm_punisher_selected_hot_hammer_updates_debt_and_earns_worm_whacker_on_sixth_hit() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, CARRION, false)
    })
}

#[test]
fn wormholes_rotate_on_owner_clock_and_reject_a_stale_carrion_handle() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(ROTATING, || {
        super::quest_world_callbacks_fixtures::scenario(ROTATING, CARRION, false)
    })
}

#[test]
fn worm_punisher_heats_fences_expires_and_reheats_the_same_carried_item() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(HEATING, || {
        super::quest_world_callbacks_fixtures::scenario(HEATING, CARRION, false)
    })
}
