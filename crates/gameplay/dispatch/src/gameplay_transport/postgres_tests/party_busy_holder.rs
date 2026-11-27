//! MP01: named visibility and party invites when the shared holder is busy.
use crate::bestiary_postgres_harness::TestResult;

#[test]
fn nbmp0_party_invite_retries_busy_holder_through_real_caller() -> TestResult {
    super::party_membership_fixtures::run_busy_holder()
}

#[test]
fn p11_staged_party_member_joins_after_transient_visibility_failure_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::party_membership_fixtures::run_join_visibility(admin)
}
