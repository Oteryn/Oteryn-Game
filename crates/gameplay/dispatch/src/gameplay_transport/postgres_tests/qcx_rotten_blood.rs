//! Global Rotten Charge cadence through the native World think and durable quest clock.
use super::TestResult;
use super::srx3_rotten_area_think_fixtures::{RottenWorldCase, run_rotten_world_case};

#[test]
fn passive_rotten_charge_advances_after_twenty_seconds() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run_rotten_world_case(RottenWorldCase::ChargeInterval)
}

#[test]
fn bakragore_admission_opens_a_twenty_minute_fight() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::srx3_rotten_advanced_fixtures::bakragore_deadline_case()
}

#[test]
fn torkada_expedition_dialogue_grants_the_temple_blessing() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run_rotten_world_case(RottenWorldCase::TorkadaExpedition)
}
