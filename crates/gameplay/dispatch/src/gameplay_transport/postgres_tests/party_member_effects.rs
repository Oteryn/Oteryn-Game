//! NX-8 S13c: a live party query publishes effects of every local party member.
use crate::bestiary_postgres_harness::TestResult;


#[test]
fn party_member_effects_real_query_durations_and_legacy_gate_postgres() -> TestResult {
    super::party_buffs_fixtures::run_party_buffs(None)
}
