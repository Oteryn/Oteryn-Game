//! SUMMER decision 20: shared corpse owner capacity, mint replay and full drain.
use super::TestResult;

use super::kill_reward_e2e_fixtures::{scenario_thread, scenario_with_capacity};

#[test]
fn summer_corpse_capacity_thirty_entries_mint_replay_and_dispatch_decay() -> TestResult {
    scenario_thread::run("summer-corpse-capacity", || {
        scenario_with_capacity(Some(false), true)
    })
}

#[test]
fn summer_corpse_capacity_thirty_entries_reconciles_missed_runtime_drain() -> TestResult {
    scenario_thread::run("summer-corpse-reconcile", || {
        scenario_with_capacity(Some(true), true)
    })
}
