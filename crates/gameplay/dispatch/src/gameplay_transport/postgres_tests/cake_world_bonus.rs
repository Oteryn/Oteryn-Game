use super::party_xp_fixtures::{Case, run_with_admin};
use crate::bestiary_postgres_harness::TestResult;

#[test]
fn pcake_world_bonus_expired_real_creature_death_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_with_admin(Case::CakeWorldBonusExpired, admin)
}

#[test]
fn pcake_world_bonus_real_creature_death_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_with_admin(Case::CakeWorldBonus, admin)
}
