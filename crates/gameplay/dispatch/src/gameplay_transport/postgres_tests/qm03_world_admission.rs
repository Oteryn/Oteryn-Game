//! M03: native USE and STEP entrances consume live, fenced admission predicates.
use super::qm03_world_admission_fixtures::{callback};

use super::TestResult;


#[test]
fn qm03_native_inferno_step_refuses_without_live_premium() -> TestResult {
    scenario("qm03/pits")
}

#[test]
fn qm03_native_inferno_step_commits_with_live_premium() -> TestResult {
    scenario("qm03/pits-premium")
}

#[test]
fn qm03_native_anniversary_use_requires_live_blessings_promotion_and_premium() -> TestResult {
    scenario("qm03/anniversary-premium")
}

fn scenario(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(
            name,
            callback(name)?
                .selector
                .position
                .ok_or("entrance position")?,
            false,
        )
    })
}
