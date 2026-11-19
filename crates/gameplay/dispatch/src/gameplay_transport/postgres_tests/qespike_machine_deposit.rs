//! Crystal 201189f8 data-global/scripts/actions/other/enchanting.lua:6-10,47-57.
use super::TestResult;
use super::qespike_machine_deposit_fixtures::{MACHINE, PREFIX};

#[test]
fn sphere_gem_deposit_fences_base_vocation_and_consumes_one_selected_gem() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(PREFIX, || {
        for vocation in ["paladin", "elite_knight"] {
            super::quest_world_callbacks_fixtures::scenario(
                &format!("{PREFIX}{vocation}"),
                MACHINE,
                false,
            )?;
        }
        Ok(())
    })
}
