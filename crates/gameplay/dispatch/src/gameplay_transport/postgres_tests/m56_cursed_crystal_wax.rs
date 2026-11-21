//! M56: framed self USE consumes exactly the selected wax and commits ear protection.

use super::TestResult;

#[test]
fn m56_cursed_crystal_exact_self_wax_consumption_and_protection_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m56-wax")
}

#[test]
fn m56_cursed_crystal_wax_outside_crystal_area_is_preserved_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m56-wax-outside")
}

#[test]
fn m56_cursed_crystal_wax_at_area_corner_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m56-wax-corner")
}

#[test]
fn m56_cursed_crystal_wax_one_tile_beyond_area_is_preserved_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m56-wax-edge-outside")
}
