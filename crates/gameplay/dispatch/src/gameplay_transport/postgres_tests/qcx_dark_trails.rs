//! Dark Trails spoiler (curid 67963, oldid 1186618), abandoned sewers dialogue.
use super::TestResult;

#[test]
fn qcx_dark_trails_sewer_dialogue_matches_global() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx-dark-trails", || {
        super::dark_trails::scenario(true)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcx_dark_trails_sewer_dialogue_matches_global duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
