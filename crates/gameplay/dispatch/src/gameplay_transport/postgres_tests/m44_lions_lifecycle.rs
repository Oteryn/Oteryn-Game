//! Native offering and pagoda are separate source objects, committed through USE.
//! OTS_HYPOTHESIS_ONLY: Crystal 201189f actions_lions_rock.lua:14-84.
use super::m44_lions_lifecycle_fixtures::{AT};

use super::TestResult;

#[test]
fn m44_native_lions_first_offering_consumption_progress_and_fenced_pagoda_reset() -> TestResult {
    case("first")
}
#[test]
fn m44_native_lions_later_offering_preserves_existing_progress_and_resets_pagoda() -> TestResult {
    case("later")
}
#[test]
fn m44_native_lions_unknown_commit_reserves_both_objects_until_reconciled() -> TestResult {
    case("first-uncertain")
}
fn case(phase: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("m44-lions", move || {
        super::quest_world_callbacks_fixtures::scenario(&format!("m44-lions-strength/{phase}"), AT, false)
    })
}
