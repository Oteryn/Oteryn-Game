//! M44: native source USE, durable commit latency and revision-fenced reset.
//! OTS_HYPOTHESIS_ONLY: Crystal 201189f871d23597b82caf88fd611454a2ddefb8,
//! data-global/scripts/quests/an_uneasy_alliance/actions_crystal_ball.lua:1-24.
use super::m44_source_lifecycle_fixtures::{AT};

use super::TestResult;

const NAME: &str = "uneasy-alliance-crystal";

#[test]
fn m44_native_source_reset_starts_after_commit_and_fences_newer_revisions() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("m44-source-lifecycle", || {
        super::quest_world_callbacks_fixtures::scenario(NAME, AT, false)
    })
}
