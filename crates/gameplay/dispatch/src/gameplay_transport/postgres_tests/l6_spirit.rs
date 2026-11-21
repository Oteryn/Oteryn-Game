//! Selected Spirit story: all three discovered wills are retained; no outfit grant.

use super::TestResult;

#[test]
fn l6_spirit_three_retained_wills_unlock_only_confession() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    eprintln!("POSTGRES_CASE_STARTED l6_spirit");
    super::qengine_rewards_fixtures::scenario("qengine-rewards-questpc4-spirit")
}
