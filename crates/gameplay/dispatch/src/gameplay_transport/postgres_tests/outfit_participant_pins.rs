//! Native NPC rewards can advance a foreign Outfit row saved on main.
use super::TestResult;

#[test]
fn amber_starts_backpack_with_unfinished_main_outfit_pin() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l6-amber-main-outfit-pin")
}

#[test]
fn hjaern_grants_norseman_with_unfinished_main_outfit_pin() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l5-hjaern-main-outfit-pin")
}
