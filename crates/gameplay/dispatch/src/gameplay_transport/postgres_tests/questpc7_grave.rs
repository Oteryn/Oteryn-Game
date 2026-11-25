//! Whole selected Grave Danger journey through native world and death owners.

use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn questpc7_grave_whole_native_journey_and_reward() -> TestResult {
    grave_case("questpc7-grave")
}

#[test]
fn questpc7_grave_jack_reports_after_zelos_without_crossing() -> TestResult {
    grave_case("questpc7-grave/after-kill")
}

#[test]
fn questpc7_grave_jack_reports_with_a_legacy_commission() -> TestResult {
    grave_case("questpc7-grave/legacy")
}

fn grave_case(case: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-grave", || {
        super::quest_world_callbacks_fixtures::scenario(
            case,
            Position {
                x: 33221,
                y: 31670,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_GRAVE POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn review_scarlett_awards_a_returning_player() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-grave-completed-scarlett", || {
        super::quest_world_callbacks_fixtures::scenario(
            "questpc7-grave-completed-scarlett",
            Position { x: 33221, y: 31670, floor: -7 },
            false,
        )
    })
}
