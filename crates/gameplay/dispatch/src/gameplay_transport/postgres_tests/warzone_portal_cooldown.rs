//! Crystal 201189f8, dangerous_depth/movements_boss_entrance.lua:10-47.
use super::TestResult;
use super::warzone_portal_cooldown_fixtures::{
    CASE, COUNT_ENTRY, COUNT_VICTORY, DUKE_ENTRY, DUKE_VICTORY, VICTORY, run_entry, run_victory,
};

#[test]
fn warzone_baron_death_redirects_the_exact_exit_and_restores_it_after_twenty_minutes() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_victory(VICTORY)
}

#[test]
fn warzone_count_exit_teleports_two_players_to_the_same_landing_and_restores() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_victory(COUNT_VICTORY)
}

#[test]
fn warzone_duke_exit_teleports_two_players_facing_west_and_restores() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_victory(DUKE_VICTORY)
}

#[test]
fn warzone_step_stamps_each_entrant_and_refuses_a_retry_without_summoning_another_boss()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_entry(CASE)
}

#[test]
fn warzone_count_entrance_stamps_each_players_count_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_entry(COUNT_ENTRY)
}

#[test]
fn warzone_duke_entrance_stamps_each_players_duke_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_entry(DUKE_ENTRY)
}
