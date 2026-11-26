//! A committed join reaches both peers while unrelated admission work holds a lock.
use crate::bestiary_postgres_harness::TestResult;

#[test]
fn party_leader_receives_joined_when_peer_joins_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    super::party_membership_fixtures::run_leader_joined_notice(admin)
}
