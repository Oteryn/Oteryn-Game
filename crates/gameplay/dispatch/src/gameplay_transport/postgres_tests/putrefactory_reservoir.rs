//! Crystal 201189f8, rotten_blood_quest/creaturescripts_putrefactory-x.lua:155-224.
use super::TestResult;
use super::putrefactory_reservoir_fixtures::{
    AT, CAP, GATE, GATE_POSITION, LEGACY, NATIVE, NATIVE_GATE, PASSIVE,
};
use super::quest_world_callbacks_fixtures::scenario;

#[test]
fn putrefactory_reservoir_recharges_unset_counter_and_cools_down_on_use() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(LEGACY, || scenario(LEGACY, AT, false))
}

#[test]
fn putrefactory_reservoir_caps_charge_without_restarting_clock_when_full() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(CAP, || scenario(CAP, AT, false))
}

#[test]
fn putrefactory_native_reservoir_recharges_without_a_startup_action_id() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(NATIVE, || scenario(NATIVE, AT, false))
}

#[test]
fn putrefactory_passive_charge_ticks_from_the_channel_clock() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(PASSIVE, || scenario(PASSIVE, AT, false))
}

#[test]
fn putrefactory_entrance_requires_full_charge_and_keeps_it_for_reentry() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(GATE, || {
        scenario(GATE, GATE_POSITION, false)
    })
}

#[test]
fn putrefactory_untagged_entrance_requires_full_charge_and_keeps_it_for_reentry() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(NATIVE_GATE, || {
        scenario(NATIVE_GATE, GATE_POSITION, false)
    })
}
