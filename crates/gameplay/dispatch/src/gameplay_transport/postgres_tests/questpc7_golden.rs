//! The selected Golden commission starts fresh and pays each exact milestone.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc7_golden_full_commission_charges_exactly_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-golden", || {
        super::quest_world_callbacks_fixtures::scenario(
            "questpc7-golden", Position { x: 33294, y: 31677, floor: -15 }, false,
        )
    });
    eprintln!("QUESTPC7_GOLDEN POSTGRES_CASE_COMPLETED duration={:?} success={}", started.elapsed(), result.is_ok());
    result
}
