//! NIGHT-W1 decision 11 through a real auto-attack kill and the server owner timer.
use super::TestResult;

use super::kill_reward_e2e_fixtures::{scenario_thread, scenario_with_decay};

#[test]
fn corpse_decay_real_kill_timer_retires_contents_preserves_ordinary_ground() -> TestResult {
    scenario_thread::run("corpse-decay-dispatch", || scenario_with_decay(Some(false)))
}

#[test]
fn corpse_decay_timer_reconciles_retired_corpse_after_missed_runtime_update() -> TestResult {
    scenario_thread::run("corpse-decay-reconcile", || scenario_with_decay(Some(true)))
}
