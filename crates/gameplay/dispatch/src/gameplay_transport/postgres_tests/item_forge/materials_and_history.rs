//! Current native stash, recursive carried custody and the shared bounded history.
use super::*;

#[test]
fn forge_postgres_conversion_consumes_carried_and_stashed_slivers_once() -> TestResult {
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("forge-stash", || {
        support::scenario(Case::Stash)
    })
}

#[test]
fn forge_postgres_conversion_consumes_materials_in_recursive_carried_custody() -> TestResult {
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("forge-nested", || {
        support::scenario(Case::Nested)
    })
}

#[test]
fn forge_postgres_history_keeps_shared_thousand_entries_and_fifty_entry_pages() -> TestResult {
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("forge-history-bound", || {
        support::scenario(Case::HistoryBound)
    })
}
