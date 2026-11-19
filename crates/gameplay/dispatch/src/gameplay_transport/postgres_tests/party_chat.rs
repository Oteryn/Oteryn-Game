//! Party chat through the real connection, Party owner, PostgreSQL relay and egress custody.
use super::party_membership_fixtures as party_membership;
use crate::bestiary_postgres_harness::TestResult;

#[test]
fn party_chat_postgres_real_wire_members_only_departure_refusal_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    party_membership::run_chat()
}
