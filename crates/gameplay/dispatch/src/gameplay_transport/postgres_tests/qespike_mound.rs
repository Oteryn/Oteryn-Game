//! Crystal 201189f8 data-global/scripts/quests/spike_tasks/actions_spirit_shovel.lua:1-43.
use super::TestResult;
use super::qespike_mound_fixtures::{LOST_ACK, MOUND, NAME, OCCUPIED};

#[test]
fn spike_mound_draw_transforms_restores_spawns_and_debits_only_on_enraged_soul() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, MOUND, false)
    })
}

#[test]
fn spike_mound_enraged_soul_roll_still_credits_when_the_spawn_tile_is_occupied() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(OCCUPIED, || {
        super::quest_world_callbacks_fixtures::scenario(OCCUPIED, MOUND, false)
    })
}

#[test]
fn spike_mound_reconciliation_publishes_both_winning_messages_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(LOST_ACK, || {
        super::quest_world_callbacks_fixtures::scenario(LOST_ACK, MOUND, false)
    })
}
