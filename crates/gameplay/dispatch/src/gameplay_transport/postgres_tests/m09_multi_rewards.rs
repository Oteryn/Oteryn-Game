//! M09: source-authored independent outputs through the production NPC wire dispatcher.

use super::TestResult;

#[test]
fn m09_tower_defence_four_rewards_once_and_replay() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m09-tower-once")
}

#[test]
fn m09_tower_defence_three_free_slots_reject_whole_reward() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-m09-tower-full")
}
