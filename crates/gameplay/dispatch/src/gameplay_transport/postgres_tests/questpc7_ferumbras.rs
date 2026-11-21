//! Selected Ferumbras route and atomic world rewards over existing fenced owners.

use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn questpc7_ferumbras_full_native_seals_and_reward_bundle() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-ferumbras", || {
        super::quest_world_callbacks_fixtures::scenario(
            "questpc7-ferumbras",
            Position {
                x: 33275,
                y: 32390,
                floor: -8,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_FERUMBRAS POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
