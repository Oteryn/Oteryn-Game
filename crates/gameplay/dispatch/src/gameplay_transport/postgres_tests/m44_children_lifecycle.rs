//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8, OTS_HYPOTHESIS_ONLY:
//! data-global/scripts/quests/children_of_the_revolution/actions_mission4_levers_puzzle.lua.
use super::m44_children_lifecycle_fixtures::{AT};

use super::TestResult;

#[test]
fn m44_native_children_readset_reset_manual_close_reopen_and_stale_timer() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("m44-children-lifecycle", || {
        super::quest_world_callbacks_fixtures::scenario("qn1-children-lever-1/ordinary", AT, false)
    })
}
