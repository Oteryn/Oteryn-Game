//! Use the Serpentine Tower lever through production dispatch.
use super::TestResult;
use super::serpentine_tower_fixtures::{LEVER, NAME};

#[test]
fn serpentine_lever_opens_and_restores_all_four_walls() -> TestResult {
    run(NAME)
}

#[test]
fn serpentine_exhausted_close_keeps_ground_custody() -> TestResult {
    run("serpentine-tower-lever#exhausted")
}

#[test]
fn serpentine_full_landing_rolls_back_all_ground_moves() -> TestResult {
    run("serpentine-tower-lever#capacity")
}

#[test]
fn serpentine_relocates_ground_drops_after_content_only_repin() -> TestResult {
    run("serpentine-tower-lever#repin")
}

fn run(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(name, LEVER, false)
    })
}
