//! Source callbacks exercise the real committed STEP ingress. The fixture supplies
//! explicit source tags; this is not evidence of tags in the shipped world.
use super::*;

#[test]
fn quests6_stateless_portals_run_through_literal_step_ingress() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("quests6-step-portals", || {
        for (key, trigger) in [
            ("galthen-iksupan-exit-1", Position { x: 34015, y: 31892, floor: -8 }),
            ("waterfall-cave-entrance", Position { x: 32968, y: 32629, floor: -7 }),
        ] {
            scenario(key, trigger, false)?;
        }
        println!("QUESTS6_STATELESS_PORTALS POSTGRES_CASE_COMPLETED");
        Ok(())
    })
}
