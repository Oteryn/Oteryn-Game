//! Dangerous Depths daily task cooldown through the live NPC dispatch path.
use super::TestResult;

#[test]
fn ordnance_can_be_accepted_again_after_twenty_hours() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::qengine_rewards_fixtures::scenario("questpc2-gnomus-ordnance")
}
