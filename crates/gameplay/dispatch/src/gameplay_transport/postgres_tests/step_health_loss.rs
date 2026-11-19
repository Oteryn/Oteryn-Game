//! Crystal 201189f8, rotten_blood_quest/movements_blood_entrance.lua:7-44.
//! The donor entrance and the fixture ingress beside the Putrefactory landing are separate selectors.
use super::TestResult;
use super::quest_world_callbacks_fixtures::scenario;
use super::step_health_loss_fixtures::{
    BELOW_LEVEL, DONOR_ENTRANCE, HEALTH_FLOOR, INGRESS, NAME, PAID, SOURCE_ENTRANCE,
};

#[test]
fn unpaid_vestibule_walk_in_relocates_and_applies_agony_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || scenario(NAME, INGRESS, false))
}

#[test]
fn unpaid_vestibule_walk_in_relocates_without_damage_at_health_floor() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(HEALTH_FLOOR, || {
        scenario(HEALTH_FLOOR, INGRESS, false)
    })
}

#[test]
fn paid_vestibule_walk_in_admits_without_agony() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(PAID, || scenario(PAID, INGRESS, false))
}

#[test]
fn vestibule_returns_below_level_player_to_landing_before_penalty() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(BELOW_LEVEL, || {
        scenario(BELOW_LEVEL, INGRESS, false)
    })
}

#[test]
fn unpaid_blood_entrance_uses_the_retained_donor_placement() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(DONOR_ENTRANCE, || {
        scenario(DONOR_ENTRANCE, SOURCE_ENTRANCE, false)
    })
}
