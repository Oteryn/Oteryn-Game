//! M48: real native mammoth USE reads the live actor's Drunk condition.
use super::qm48_mammoth_fixtures::{AT};

use super::TestResult;

#[test]
fn qm48_mammoth_live_drunk_refusal_expiry_cleanse_commit_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("qm48-mammoth", || {
        super::quest_world_callbacks_fixtures::scenario("qm48/mammoth", AT, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED qm48_mammoth duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
