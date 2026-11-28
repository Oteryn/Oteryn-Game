use super::TestResult;
use super::quest_hunter_hymn_fixtures::PIANO;

#[test]
fn hunter_hymn_consumes_each_verse_once_and_opens_the_piano_route() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("hunter-hymn", || {
        super::quest_world_callbacks_fixtures::scenario("hunter-hymn-piano", PIANO, false)
    })
}
