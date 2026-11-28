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

#[test]
fn qc_twenty_miles_hoard_commit_survives_control_loss() -> TestResult {
    run(quest::LOSS)
}

#[test]
fn qc_twenty_miles_merged_gold_recovery() -> TestResult {
    run(quest::GOLD)
}

#[test]
fn qc_twenty_miles_split_gold_recovery() -> TestResult {
    run(quest::SPLIT_GOLD)
}

#[test]
fn qc_twenty_miles_ignores_splits_before_gold_arrives() -> TestResult {
    run(quest::EARLIER_SPLIT_GOLD)
}

#[test]
fn qc_twenty_miles_world_use_refuses_open_trade() -> TestResult {
    run(quest::TRADE)
}

#[test]
fn qc_twenty_miles_wandering_guardians_keep_dialogue() -> TestResult {
    run(quest::GUARDIAN)
}

#[test]
fn qc_twenty_miles_first_shell_pickup_after_borrowed_report() -> TestResult {
    run(quest::BORROWED_REPORT)
}

#[test]
fn qc_twenty_miles_first_shell_pickup_after_borrowed_songs() -> TestResult {
    run(quest::BORROWED_SONGS)
}

fn run(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(name, quest::START, false)
    })
}
