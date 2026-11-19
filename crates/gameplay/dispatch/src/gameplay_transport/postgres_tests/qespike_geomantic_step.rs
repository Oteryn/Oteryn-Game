//! Crystal 201189f8 data-global/scripts/quests/the_spike_tasks/movements_geomantic_charges.lua:1-45.
use super::TestResult;
use super::qespike_geomantic_step_fixtures::{CELL, CHARGE_ONLY, NAME};

#[test]
fn spike_geomantic_step_checks_outfit_before_charge_then_applies_both() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, CELL, false)
    })
}

#[test]
fn spike_geomantic_step_charges_without_nest_and_refuses_after_charge() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(CHARGE_ONLY, || {
        super::quest_world_callbacks_fixtures::scenario(CHARGE_ONLY, CELL, false)
    })
}
