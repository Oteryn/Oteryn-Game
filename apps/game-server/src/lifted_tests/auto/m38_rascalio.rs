//! The production conditional actor consumes the durable event owner's current occurrence.
#[test]
fn m38_rascalio_postgres_event_presence_preserves_other_quest_occurrences()
-> crate::bestiary_postgres_harness::TestResult {
    crate::node::serve::bundle_npcs::tests::m38_rascalio_postgres_case()
}
