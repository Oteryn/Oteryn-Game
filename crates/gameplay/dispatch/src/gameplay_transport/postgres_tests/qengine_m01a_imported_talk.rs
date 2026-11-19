//! M01a imported chosen-recipe admission and real NPC command execution.
//! A binding fixture, not an end-to-end qualification of the Yalahar journey.

use super::TestResult;

#[test]
fn m01a_imported_recipe_native_talk_dispatch_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-authored-imported-talk")
}

#[test]
fn m01a_multi_npc_talk_stage_is_atomic_and_replay_bound_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-authored-imported-talk-group")
}
