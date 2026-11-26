use crate::bestiary_postgres_harness::TestResult;

#[test]
fn party_invite_and_disband_push_only_to_intended_live_peers() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::party_membership_fixtures::run_invite_disband_notices(admin)
}
