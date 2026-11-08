//! Auto-mounted PG cases share the existing fenced reward-claim fixture.
use super::reward_claim_mint_postgres_cases::qmech_siblings;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn qmech_siblings_exact_once_replay_and_changed_intent() -> TestResult {
    qmech_siblings::qmech_siblings_exact_once_replay_and_changed_intent()
}

#[test]
fn qmech_siblings_full_bag_writes_no_partial_reward() -> TestResult {
    qmech_siblings::qmech_siblings_full_bag_writes_no_partial_reward()
}

#[test]
fn qmech_siblings_weight_counts_all_outputs_before_claim() -> TestResult {
    qmech_siblings::qmech_siblings_weight_counts_all_outputs_before_claim()
}

#[test]
fn qmech_siblings_incomplete_source_is_an_integrity_refusal() -> TestResult {
    qmech_siblings::qmech_siblings_incomplete_source_is_an_integrity_refusal()
}
