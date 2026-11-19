//! Real world USE joins an existing source wardrobe to the fenced cosmetic owner.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc7_world_wardrobe_grant_is_atomic_and_replay_safe() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let start = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-world-outfit", || {
        super::quest_world_callbacks_fixtures::scenario(
            "l1-warmaster-wardrobe",
            Position {
                x: 33053,
                y: 31020,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_WORLD_OUTFIT POSTGRES_CASE_COMPLETED duration={:?} success={}",
        start.elapsed(),
        result.is_ok()
    );
    result
}
