//! Crystal 201189f8, ferumbras_ascension/actions_ferumbras_lever.lua:1-62.
use super::TestResult;
use super::ferumbras_crystal_reset_fixtures::{CASE, LEVER};

#[test]
fn ferumbras_crystal_charges_and_seven_vortex_steps_reach_the_mortal_shell() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(CASE, || {
        super::quest_world_callbacks_fixtures::scenario(CASE, LEVER, false)
    })
}
