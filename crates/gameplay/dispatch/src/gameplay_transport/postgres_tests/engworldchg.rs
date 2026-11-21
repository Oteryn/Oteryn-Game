//! Current native binding decisions precede the real Grimvale STEP and Hive USE.
use super::TestResult;

#[test]
fn engworldchg_grimvale_binding_restart_and_native_step_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("engworldchg-grimvale", || {
        super::worldchg_grimvale_fixtures::scenario_with_bindings(true)
    })
}

#[test]
fn engworldchg_hive_binding_reset_and_native_use_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("engworldchg-hive", || {
        super::worldchg_hive_egress_fixtures::scenario_with_bindings(true)
    })
}
