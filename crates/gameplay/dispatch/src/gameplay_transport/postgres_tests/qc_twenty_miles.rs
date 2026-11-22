use super::{TestResult, qc_twenty_miles_fixtures as quest};

#[test]
fn qc_twenty_miles_pinned_selectors_and_npcs() -> TestResult {
    quest::pinned_placements().map(|_| ())
}

#[test]
fn qc_twenty_miles_live_chain() -> TestResult {
    run(quest::CHAIN)
}

#[test]
fn qc_twenty_miles_resumes_the_pre_withdrawal_definition() -> TestResult {
    run(quest::RESUME)
}

fn run(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(name, quest::START, false)
    })
}
