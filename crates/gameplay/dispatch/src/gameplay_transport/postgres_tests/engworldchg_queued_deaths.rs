//! World-change hunts preserve every queued death without accepting stale activations.
use super::TestResult;

#[test]
fn engworldchg_queued_fury_native_deaths_postgres() -> TestResult {
    super::qworld_native_deaths::run("fury")
}

#[test]
fn engworldchg_queued_werewolf_native_deaths_postgres() -> TestResult {
    super::qworld_native_deaths::run("werewolf")
}
