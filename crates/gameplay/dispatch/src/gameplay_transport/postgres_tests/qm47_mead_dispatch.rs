//! M47: literal USE of the native barrel reaches the durable random-outcome owner.
use super::qm47_mead_dispatch_fixtures::{AT};

use super::TestResult;

#[test]
fn m47_native_use_commits_random_outcome_feedback_blackout_and_replays() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("m47-native-mead", || {
        super::quest_world_callbacks_fixtures::scenario("barbarian-test-mead-native", AT, false)
    })
}
