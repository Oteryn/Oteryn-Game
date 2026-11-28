//! A native Rat kill, followed by the Channel owner timer from serving.
use super::TestResult;
use super::kill_reward_e2e_fixtures::{scenario_thread, scenario_with_rat_drag, scenario_with_rat_successor, scenario_with_rat_transform};

#[test]
fn rat_corpse_owner_tick_commits_first_decay_transform() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin); // The shared scenario opens and cleans up its isolated database.
    scenario_thread::run("rat-corpse-transform", scenario_with_rat_transform)
}


#[test]
fn rat_corpse_floor_drag_preserves_projection_loot_and_terminal_decay_after_restart() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario_thread::run("rat-corpse-drag", scenario_with_rat_drag)
}


#[test]
fn rat_corpse_successor_quick_loots_and_drags_before_next_owner_decays() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario_thread::run("rat-corpse-successor", scenario_with_rat_successor)
}
