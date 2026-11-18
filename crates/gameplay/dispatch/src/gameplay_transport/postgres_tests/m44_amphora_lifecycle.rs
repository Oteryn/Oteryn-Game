//! Crystal 201189f movements_lions_rock.lua:54-108, OTS_HYPOTHESIS_ONLY.
//! Literal STEP must couple the scroll and questline to the separate amphora.
use super::m44_amphora_lifecycle_fixtures::{AT};

use super::TestResult;

#[test]
fn m44_native_amphora_step_mints_once_and_restores_only_its_fenced_revision() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("m44-amphora", || {
        super::quest_world_callbacks_fixtures::scenario("m44-lions-amphora-1", AT, false)
    })
}
