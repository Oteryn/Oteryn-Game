//! Crystal 201189f actions_lions_rock.lua:82-96, OTS_HYPOTHESIS_ONLY.
//! Real USE couples each native regrowth revision to one flower MINT.
use super::m44_flower_lifecycle_fixtures::{AT};

use super::TestResult;

#[test]
fn m44_native_flower_initial_harvest_and_reharvest_after_exact_regrowth() -> TestResult {
    case("initial")
}
#[test]
fn m44_native_flower_reharvest_preserves_higher_quest_progress() -> TestResult {
    case("later")
}
#[test]
fn m44_native_flower_unknown_commit_reserves_source_and_reconciles_one_grant() -> TestResult {
    case("uncertain")
}
fn case(phase: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("m44-flower", move || {
        super::quest_world_callbacks_fixtures::scenario(&format!("m44-lions-flower-0/{phase}"), AT, false)
    })
}
