//! Native two-floor winches through the real USE transport and Channel owners.

use super::TestResult;

use oteryn_gameplay_quests::quest::{goblin_merchant as elevator, world_callbacks::Position};

#[test]
fn qm50_native_elevator_call_ride_and_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("qm50-elevator", || {
        super::quest_world_callbacks_fixtures::scenario(
            "goblin-merchant-elevator-upper",
            Position {
                x: 33052,
                ..elevator::UPPER
            },
            false,
        )
    })
}
