use super::party_xp_fixtures::{Case, run_with_admin};
use crate::bestiary_postgres_harness::TestResult;

#[test]
fn pcake_kill_before_activation_drains_in_window_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_with_admin(Case::CakeKillBeforeActivation, admin)
}

#[test]
fn pcake_kill_in_window_drains_after_expiry_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_with_admin(Case::CakeKillBeforeExpiry, admin)
}

#[test]
fn pcake_regeneration_ticks_inside_house_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::hireling_lifecycle_fixtures::run_case_with_admin(
        super::hireling_lifecycle_fixtures::Case::CakeHouseRegeneration,
        admin,
    )
}

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
