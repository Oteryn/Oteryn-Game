//! Source Lion trial admission through real USE and the durable Quest/Item owners.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn m07_lion_strength_offering_and_shared_reset() -> TestResult {
    trial("lions-rock-strength-first", Position { x:33137, y:32291, floor:-8 })
}

#[test]
fn m07_lion_beauty_offering_and_shared_reset() -> TestResult {
    trial("lions-rock-beauty", Position { x:33138, y:32369, floor:-8 })
}

#[test]
fn m07_lion_tears_offering_and_shared_reset() -> TestResult {
    trial("lions-rock-tears", Position { x:33154, y:32279, floor:-8 })
}

fn trial(name: &'static str, position: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("m07-lion-trial", move || {
        super::quest_world_callbacks_fixtures::scenario(name, position, false)
    })
}
