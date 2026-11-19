//! Crystal 201189f8, data-global/scripts/quests/the_first_dragon/
//! movements_entrance_teleport.lua:41-87 and data/libs/functions/functions.lua:226-239.
use super::TestResult;
use super::first_dragon_descendant_fixtures::{
    CASE, GELIDRAZAH, KALYASSA, PORTAL, SWEEP_CASE, ZORVORAX, run,
};

#[test]
fn tazhadur_step_summons_once_stamps_boss_cooldown_and_ejects_at_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(CASE, || {
        super::quest_world_callbacks_fixtures::scenario(CASE, PORTAL, false)
    })
}

#[test]
fn tazhadur_deadline_sweeps_late_monsters_but_only_ejects_the_admitted_player() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(SWEEP_CASE)
}

#[test]
fn kalyassa_uid_accepts_chest_progress_above_four_and_stamps_its_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(KALYASSA)
}

#[test]
fn zorvorax_uid_admits_the_three_secrets_route_and_clears_at_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(ZORVORAX)
}

#[test]
fn gelidrazah_uid_admits_the_access_flag_and_clears_at_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(GELIDRAZAH)
}
