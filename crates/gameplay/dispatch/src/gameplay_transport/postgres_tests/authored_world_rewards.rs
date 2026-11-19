//! Synthetic engine fixture: no acquisition story or packet quest is qualified here.
//! The retained Behemoth callback supplies valid template provenance only.

use super::TestResult;

#[test]
fn authored_world_use_item_delivery_is_atomic_and_replay_bound_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-authored-world-rewards")
}

#[test]
fn authored_world_full_backpack_rolls_back_reward_and_quest_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-authored-world-rewards-full")
}

#[test]
fn authored_native_boot_recipe_admission_dispatch_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-authored-native-admission")
}
