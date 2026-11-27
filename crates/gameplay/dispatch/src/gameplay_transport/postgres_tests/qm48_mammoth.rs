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

#[test]
fn qm48_mammoth_lost_ack_reconciles_the_delayed_cue_once() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("qm48-mammoth-recovery", || {
        super::quest_world_callbacks_fixtures::scenario("qm48/mammoth-recovery", AT, false)
    })
}

#[test]
fn barbarian_test_fills_horn_and_hugs_only_the_stunned_source_bear() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    use super::qm48_mammoth_fixtures::{BARREL, BEAR};
    super::kill_reward_e2e_fixtures::scenario_thread::run("barbarian-test-horn", || {
        super::quest_world_callbacks_fixtures::scenario("barbarian-test/fill-horn", BARREL, false)
    })?;
    super::kill_reward_e2e_fixtures::scenario_thread::run("barbarian-test-bear", || {
        super::quest_world_callbacks_fixtures::scenario("barbarian-test/stun-bear", BEAR, false)
    })
}
