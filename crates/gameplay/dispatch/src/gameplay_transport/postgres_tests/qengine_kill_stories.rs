//! Hunting trials must count real native creature deaths, not fixture quest writes.

use super::TestResult;

#[test]
fn qengine_l12_dragon_slayer_native_trial_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l12-dragon-slayer-story")
}
#[test]
fn qengine_l12_cave_explorer_native_trial_story() -> TestResult {
    super::qengine_rewards_fixtures::scenario("qengine-rewards-l12-cave-explorer-story")?;
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("spike-nest-recovery", || {
        super::quest_world_callbacks_fixtures::scenario(
            "spike/nest-32162-32507-11",
            oteryn_gameplay_quests::quest::world_callbacks::Position { x: 32162, y: 32507, floor: -11 },
            false,
        )
    })
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

#[test]
fn spike_remaining_nest_draws_and_both_transform_pads() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    for (kind, x, y) in [
        ("nest", 32221, 32536),
        ("nest", 32112, 32664),
        ("nest", 32180, 32558),
        ("transform", 32242, 32610),
        ("transform", 32242, 32611),
    ] {
        super::kill_reward_e2e_fixtures::scenario_thread::run("spike-callback", move || {
            super::quest_world_callbacks_fixtures::scenario(
                &format!("spike/{kind}-{x}-{y}-11"),
                oteryn_gameplay_quests::quest::world_callbacks::Position { x, y, floor: -11 },
                false,
            )
        })?;
    }
    Ok(())
}
