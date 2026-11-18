//! Global Ice Islands quest compatibility through the carried-use dispatch route.
use super::TestResult;

#[test]
fn artful_sabotage_accepts_repainting_after_the_seal_resets() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx_the_ice_islands", || {
        super::quest_world_callbacks::scenario(
            "engitem-native-seal-paint",
            super::engitem_seal_paint::position(super::engitem_seal_paint::SEALS[0])?,
            false,
        )
    });
    eprintln!("QCX PG artful_sabotage duration={:?}", started.elapsed());
    result
}
