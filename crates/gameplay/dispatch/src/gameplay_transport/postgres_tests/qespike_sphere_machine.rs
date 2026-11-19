//! Crystal 201189f8 data-global/scripts/quests/elemental_spheres/actions_machine1.lua:2-37.
use super::TestResult;
use super::qespike_sphere_machine_fixtures::{
    BELOW_THRESHOLD, LEFT, LOST_ACK, LOST_ACK_SINGLE_TARGET, NAME,
};

#[test]
fn sphere_machine_dispatch_debits_twenty_and_toggles_both_halves_on_vocation_travel() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, LEFT, false)
    })
}

#[test]
fn sphere_machine_below_twenty_toggles_on_and_off_without_debit_or_travel() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(BELOW_THRESHOLD, || {
        super::quest_world_callbacks_fixtures::scenario(BELOW_THRESHOLD, LEFT, false)
    })
}

#[test]
fn sphere_machine_replays_committed_debit_and_finishes_travel_after_lost_ack() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(LOST_ACK, || {
        super::quest_world_callbacks_fixtures::scenario(LOST_ACK, LEFT, false)
    })
}

#[test]
fn sphere_machine_reserved_half_blocks_single_target_use_until_reconciled() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(LOST_ACK_SINGLE_TARGET, || {
        super::quest_world_callbacks_fixtures::scenario(LOST_ACK_SINGLE_TARGET, LEFT, false)
    })
}
