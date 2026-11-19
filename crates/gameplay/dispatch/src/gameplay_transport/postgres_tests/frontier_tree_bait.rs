//! Crystal 201189f8, data-global/scripts/quests/the_new_frontier/action_beaver.lua:12-44
//! and data-global/monster/mammals/thieving_squirrel.lua:63-65.

use super::TestResult;

#[test]
fn bait_use_spawns_a_thief_and_the_same_creature_returns_bait_on_death() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards_fixtures::scenario(super::frontier_tree_bait_fixtures::NAME)
}

#[test]
fn marked_trees_spawn_their_full_group_without_consuming_bait() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards_fixtures::scenario(super::frontier_tree_bait_fixtures::SQUIRRELS)?;
    super::qengine_rewards_fixtures::scenario(super::frontier_tree_bait_fixtures::WOLVES)
}
