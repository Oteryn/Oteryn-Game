//! The explicit eligibility query uses the committed Party after each Joined.
use crate::bestiary_postgres_harness::TestResult;

#[test]
fn party_joined_leader_eligibility_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    super::party_membership_fixtures::run_joined_eligibility(admin)?;
    eprintln!("POSTGRES_CASE_EXECUTED party_joined_leader_eligibility_postgres iterations=20");
    Ok(())
}
