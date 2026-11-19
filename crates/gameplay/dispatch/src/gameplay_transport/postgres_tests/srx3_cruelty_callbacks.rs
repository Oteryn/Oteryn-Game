//! Actual pinned Cruelty entry and native defense callback; no seeded catalogue.
use super::srx3_cruelty_callbacks_fixtures::{run_source_entry, run_source_entry_case};

use super::TestResult;

#[test]
fn srx3_cruelty_actual_source_entry_and_native_defense_chain_postgres() -> TestResult {
    run_source_entry(false)
}

#[test]
fn srx3_cruelty_canceled_before_names_drains_original_source_use_postgres() -> TestResult {
    run_source_entry_case(
        false,
        Some(super::srx3_soul_war_source_use_fixtures::RecoveryPoint::BeforeNames),
    )
}

#[test]
fn srx3_cruelty_canceled_after_write_reconciles_source_effect_once_postgres() -> TestResult {
    run_source_entry_case(
        false,
        Some(super::srx3_soul_war_source_use_fixtures::RecoveryPoint::AfterWrite),
    )
}

#[test]
fn srx3_cruelty_completed_handoff_retry_preserves_source_effect_once_postgres() -> TestResult {
    run_source_entry_case(
        false,
        Some(super::srx3_soul_war_source_use_fixtures::RecoveryPoint::Handoff),
    )
}
