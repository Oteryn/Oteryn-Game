//! Chosen four-crystal route uses exact native objects and a retained Snake Sceptre.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_wrath_four_native_crystals_require_tool_order_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-wrath-crystals", || {
        super::quest_world_callbacks_fixtures::scenario(
            "authored-wrath-crystals",
            Position {
                x: 33049,
                y: 31089,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_FINISHED questpc8_wrath_crystals duration={:?} result={result:?}",
        started.elapsed()
    );
    result
}
