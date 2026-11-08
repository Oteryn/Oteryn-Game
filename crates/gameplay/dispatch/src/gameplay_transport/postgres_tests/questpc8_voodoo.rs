//! Retained independent Voodoo regression shares the stronger cold-replay route harness.
use super::{TestResult, quest_world_callbacks};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_voodoo_three_native_witches_two_boxes_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-voodoo", || {
        quest_world_callbacks::scenario(
            "authored-voodoo",
            Position {
                x: 32743,
                y: 31950,
                floor: -9,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_voodoo_three_native_witches_two_boxes_and_replay duration={:?}",
        started.elapsed()
    );
    Ok(())
}
