//! Crystal 201189f8 dankwart.lua:60-66, fluids.lua:1-2,120-122 and action_horn.lua:34-46.
//! Buying and drinking Dankwart's mead must enable the native mammoth callback.
use super::qm48_mammoth_fixtures::{AT, NAME};

use super::TestResult;

#[test]
fn qm48_mammoth_live_drunk_refusal_expiry_cleanse_commit_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("qm48-mammoth", || {
        super::quest_world_callbacks_fixtures::scenario(NAME, AT, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED qm48_mammoth duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
