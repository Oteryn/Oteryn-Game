//! Hunting trials must count real native creature deaths, not fixture quest writes.

use super::TestResult;

#[test]
fn qengine_l12_dragon_slayer_native_trial_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l12-dragon-slayer-story")
}
#[test]
fn qengine_l12_cave_explorer_native_trial_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l12-cave-explorer-story")
}
#[test]
fn qengine_l12_fire_fighter_native_trial_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l12-fire-fighter-story")
}

#[test]
fn qengine_l12_cave_explorer_ignores_one_inherited_point() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l12-cave-explorer-inherited-one-story")
}

#[test]
fn qengine_l12_cave_explorer_grants_with_inherited_cap() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l12-cave-explorer-inherited-two-story")
}
