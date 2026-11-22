use super::TestResult;
use super::explorer_society_journey_fixtures::{NAME, START};

#[test]
fn explorer_society_full_field_course_earns_backpack_and_access() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, START, false)
    })
}
