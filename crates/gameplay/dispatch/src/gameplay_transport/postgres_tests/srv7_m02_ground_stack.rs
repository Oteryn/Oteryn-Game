//! M02: framed world USE reads the real ordered Ground projection without debit.

use super::TestResult;

const NAME: &str = "qengine-rewards-questpc4-puzzle-predicate-srv7-m02";

#[test]
fn srv7_m02_real_ground_order_guards_framed_world_use_without_consumption() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() { return Ok(()); }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("srv7_ground_stack", || {
        super::qengine_rewards_fixtures::scenario(NAME)
    });
    eprintln!("SRV7_PG_EXECUTED ground_stack duration={:?} success={}", started.elapsed(), result.is_ok());
    assert!(started.elapsed().as_secs_f64() > 0.01);
    result
}
