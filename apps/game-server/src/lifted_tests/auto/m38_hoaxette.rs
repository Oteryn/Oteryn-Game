//! The private boot controller consumes the real reset owner's PostgreSQL calendar.
#[test]
fn m38_hoaxette_postgres_reset_calendar_controls_presence_and_epoch_replay()
-> crate::bestiary_postgres_harness::TestResult {
    crate::node::serve::bundle_npcs::tests::m38_hoaxette_postgres_case()
}
