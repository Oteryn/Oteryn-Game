//! TibiaWiki Barbarian Test/Spoiler, curid 16983, oldid 1086713.
use super::{qm47_mead_dispatch, TestResult};

#[test]
fn mead_requires_ten_consecutive_successes_within_twenty_attempts() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx-barbarian-test", || {
        super::quest_world_callbacks::scenario(
            "barbarian-test-mead-native",
            qm47_mead_dispatch::AT,
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcx_barbarian_test duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
