//! Personal native deaths are admitted against the current durable world occurrence.
use super::qworld_native_deaths_fixtures::{run};

use super::*;

#[test]
fn qworld_native_fury_death_requires_active_world_and_credits_once() -> TestResult {
    run("fury")
}

#[test]
fn qworld_native_werewolf_death_requires_full_moon_and_credits_once() -> TestResult {
    run("werewolf")
}
