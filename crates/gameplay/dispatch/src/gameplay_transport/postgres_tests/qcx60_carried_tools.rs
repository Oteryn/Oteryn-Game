//! Crystal 201189f8 Ice Islands tool USE, with a synthetic map and authored bindings.
use super::TestResult;
use super::qcx60_carried_tools_fixtures::{
    ANTHILL_TARGET, CHARM_TARGET, CURE_NAME, GEYSER_TARGET, MAST_TARGET, MAST_WITH_JUG,
    MAST_WITHOUT_JUG, NAME, RETALIATION_NAME,
};

#[test]
fn ice_islands_anthill_requires_the_carried_jug_and_transforms_one() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, ANTHILL_TARGET, false)
    })
}

#[test]
fn ice_islands_geyser_transforms_the_vial_without_advancing_the_quest() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(CURE_NAME, || {
        super::quest_world_callbacks_fixtures::scenario(CURE_NAME, GEYSER_TARGET, false)
    })
}

#[test]
fn ice_islands_frost_charm_requires_the_carried_tool_and_marks_an_obelisk_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(RETALIATION_NAME, || {
        super::quest_world_callbacks_fixtures::scenario(RETALIATION_NAME, CHARM_TARGET, false)
    })
}

#[test]
fn ice_islands_mast_keeps_the_selected_filled_jug_and_debits_an_empty_jug_if_present() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(MAST_WITH_JUG, || {
        for name in [MAST_WITH_JUG, MAST_WITHOUT_JUG] {
            super::quest_world_callbacks_fixtures::scenario(name, MAST_TARGET, false)?;
        }
        Ok(())
    })
}
