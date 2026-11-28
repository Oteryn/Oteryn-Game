use super::TestResult;

#[test]
fn spike_carried_tools_admit_the_observed_site_and_selected_instance() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    scenario(Quest::Spike)
}
use super::adventurers_guild_fixtures::{Quest, scenario};

#[test]
fn great_dragon_hunt_explorer_use_starts_once_and_grants_leather() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    scenario(Quest::GreatDragonHunt)
}

#[test]
fn tarun_acceptance_remains_and_report_reward() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    scenario(Quest::LostBrother)
}
