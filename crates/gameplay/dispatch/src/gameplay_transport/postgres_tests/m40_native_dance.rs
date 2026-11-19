//! All sixteen source dance entries through native STEP; no intermediate stages seeded.
//! Crystal201189f movements_mission3_dance_dance_evolution.lua:1-67.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[path = "m40_native_dance/rollout.rs"]
mod rollout;

#[test]
fn m40_native_diagonal_dance_full_sequence_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    if rollout::receipt_process(concat!(
        module_path!(),
        "::m40_native_diagonal_dance_full_sequence_and_replay"
    ))? {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("m40-native-dance", || {
        super::quest_world_callbacks_fixtures::scenario(
            "unnatural-selection-dance-native",
            Position {
                x: 32991,
                y: 31496,
                floor: -1,
            },
            false,
        )
    })
}
