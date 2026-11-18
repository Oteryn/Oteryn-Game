use crate::bestiary_postgres_harness::TestResult;

#[test]
fn pcake_world_bonus_real_creature_death_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::party_xp::run_with_admin(super::party_xp::Case::CakeWorldBonus, admin)
}
