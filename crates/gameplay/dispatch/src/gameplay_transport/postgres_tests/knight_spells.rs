//! Native combat dispatch with current equipment, entitlement and real spell payment.
use super::{TestResult, parameter_spell_fixtures::scenario};

#[test]
fn flurry_of_blows_hits_the_aimed_wave_and_builds_harmony() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(9)
}

#[test]
fn shield_bash_hits_only_its_target_and_weakens_the_next_attack() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(10)
}

#[test]
fn shield_slam_weakens_adjacent_creatures() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(11)
}

#[test]
fn sweeping_takedown_hits_both_areas_and_spends_harmony() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(12)
}

#[test]
fn divine_empowerment_mints_owned_ground_items_that_expire_after_five_seconds() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(13)
}
