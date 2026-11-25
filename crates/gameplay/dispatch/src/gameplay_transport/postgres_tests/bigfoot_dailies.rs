use super::TestResult;
use super::bigfoot_dailies_fixtures::{EXTERMINATION, KEEPER, MATCHMAKER, contact};

#[test]
fn bigfoot_daily_crystal_keeper_dispatches_acceptance_repairs_and_report() -> TestResult {
    run(KEEPER)
}

#[test]
fn bigfoot_daily_matchmaker_dispatches_acceptance_unpacking_match_and_report() -> TestResult {
    run(MATCHMAKER)
}

#[test]
fn bigfoot_daily_extermination_dispatches_acceptance_wiggler_credit_and_report() -> TestResult {
    run(EXTERMINATION)
}

fn run(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(name, contact(name), false)
    })
}
