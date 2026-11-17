//! World-change hunts preserve every queued death without accepting stale activations.
use super::TestResult;

#[cfg(feature = "pg-tests-g2")]
#[test]
fn engworldchg_queued_fury_native_deaths_postgres() -> TestResult {
    super::qworld_native_deaths::run("fury")
}

#[cfg(feature = "pg-tests-g2")]
#[test]
fn engworldchg_queued_werewolf_native_deaths_postgres() -> TestResult {
    super::qworld_native_deaths::run("werewolf")
}
