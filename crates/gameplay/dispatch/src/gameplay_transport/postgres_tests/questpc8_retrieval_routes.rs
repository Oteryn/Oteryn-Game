//! Selected retrieval programmes use native anchors and the existing quest/MINT owners.
use super::questpc8_retrieval_routes_fixtures::Route;

use super::{TestResult, quest_world_callbacks_fixtures as quest_world_callbacks};

fn scenario(route: Route) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("questpc8-retrieval", move || {
        quest_world_callbacks::scenario(route.name(), route.anchor(), false)
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_retrieval_{} duration={:?}",
        route.slug(),
        started.elapsed()
    );
    Ok(())
}
#[test]
fn questpc8_retrieval_voodoo_three_witches_both_boxes_once() -> TestResult {
    scenario(Route::Voodoo)
}
#[test]
fn questpc8_retrieval_mastermind_two_routes_exact_outputs_once() -> TestResult {
    scenario(Route::Mastermind)
}
#[test]
fn questpc8_retrieval_kormarak_native_guardian_body_outputs_once() -> TestResult {
    scenario(Route::Kormarak)
}
