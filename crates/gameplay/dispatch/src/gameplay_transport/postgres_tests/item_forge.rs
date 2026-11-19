//! P18 command bytes → current composed owner → native rows, receipts and read-back.
use super::item_forge_fixtures::support;

use super::TestResult;


#[path = "item_forge/legacy_budget.rs"]
mod legacy_budget;
#[path = "item_forge/materials_and_history.rs"]
mod materials_and_history;

#[path = "item_forge/revision_and_guards.rs"]
mod revision_and_guards;

use support::Case;

#[test]
fn forge_postgres_dust_sliver_core_conversion_and_history_replay() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-conversions", || {
        support::scenario(Case::Conversions)
    })
}

#[test]
fn forge_postgres_dust_limit_native_receipt_history_and_cooldown() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-limit", || {
        support::scenario(Case::DustLimit)
    })
}

#[test]
fn forge_postgres_dust_limit_cap_refusal_is_retained_without_value_changes() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-cap", || support::scenario(Case::DustCap))
}

#[test]
fn forge_postgres_resource_refusals_remain_retained_after_refill() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-refusals", || {
        support::scenario(Case::Refusals)
    })
}

#[test]
fn forge_postgres_capability_payload_and_session_generation_are_fenced() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-fences", || support::scenario(Case::Fences))
}

#[test]
fn forge_postgres_concurrent_duplicate_occurrence_applies_exactly_once() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-concurrent", || {
        support::scenario(Case::Concurrent)
    })
}

#[test]
fn forge_postgres_conversion_with_full_backpack_refuses_without_spending() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("forge-no-room", || {
        support::scenario(Case::NoRoom)
    })
}
