//! Real auto attacks and durable settlement on a native profile with no corpse Item.

use crate::bestiary_postgres_harness::TestResult;

#[test]
fn corpse_free_kill_awards_shared_xp_and_credit_once_without_minting_items() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::party_xp_fixtures::run_with_admin(super::party_xp_fixtures::Case::CorpseFree, admin)
}
