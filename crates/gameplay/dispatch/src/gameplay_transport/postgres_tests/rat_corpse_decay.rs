//! A native Rat kill, followed by the Channel owner timer from serving.
use super::TestResult;
use super::kill_reward_e2e_fixtures::{scenario_thread, scenario_with_rat_chain, scenario_with_rat_transform};

#[test]
fn rat_corpse_owner_tick_commits_first_decay_transform() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin); // The shared scenario opens and cleans up its isolated database.
    scenario_thread::run("rat-corpse-transform", scenario_with_rat_transform)
}

#[test]
fn rat_corpse_owner_tick_runs_the_authored_decay_chain() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario_thread::run("rat-corpse-chain", || scenario_with_rat_chain(false))
}

#[test]
fn rat_corpse_owner_tick_retries_a_busy_holder_at_the_first_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario_thread::run("rat-corpse-chain-busy", || scenario_with_rat_chain(true))
}
