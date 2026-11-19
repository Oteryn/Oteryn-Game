//! Live level/Premium admission through the production Pits STEP binding.
use super::qengine_m03_pits_fixtures::{NAME};

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub(super) const ENTRY: Position = Position {
    x: 32791,
    y: 32331,
    floor: -10,
};

#[test]
fn m03_pits_native_step_enforces_live_premium_level_and_replay_postgres() -> TestResult {
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, ENTRY, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
