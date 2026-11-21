//! A Dreamer ring offering committed with a lost acknowledgement publishes its trees on replay.
use super::m07_dreamer_ring_lost_ack_fixtures::{NAME};

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn lost_ack_dreamer_ring_replay_republishes_trees() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(
            NAME,
            Position {
                x: 100,
                y: 100,
                floor: -11,
            },
            false,
        )
    })
}
