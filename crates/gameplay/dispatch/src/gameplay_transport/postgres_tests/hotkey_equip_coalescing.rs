//! PKG7 I17d: real command ingress, native timing and fenced equipment receipts.
use super::TestResult;

#[test]
fn hotkey_equip_postgres_coalesces_burst_without_cancelled_receipts() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-equip-coalescing", || {
        super::hotkey_equip::coalescing_scenario()
    })
}
