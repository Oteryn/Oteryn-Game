//! NX-6: the actual NPC route must commit the dream and Wayfarer together.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn nx6_wayfarer_real_npc_atomic_finale_replay_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("nx6-wayfarer", || {
        super::quest_world_callbacks_fixtures::scenario(
            "nx6-wayfarer",
            Position {
                x: 33243,
                y: 31214,
                floor: -10,
            },
            false,
        )
    })
}
