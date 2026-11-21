use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn reward_chest_grants_the_complete_bundle_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("qcx36-ferumbras-reward", || {
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
        "QCX36 reward_chest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
