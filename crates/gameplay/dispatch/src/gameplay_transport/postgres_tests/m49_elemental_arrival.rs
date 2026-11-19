//! Current command-9 arrivals and historical receipt recovery on native map crops.
use super::m49_elemental_arrival_fixtures::{rule};

use super::TestResult;

fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let trigger = rule(name)?.spawn;
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks_fixtures::scenario(name, trigger, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "M49 scenario thread")?
        .map_err(Into::into)
}
#[test]
fn m49_energy_native_arrival_summons_once() -> TestResult {
    run("m49-energy")
}
#[test]
fn m49_energy_durable_fields_summon_once() -> TestResult {
    run("m49-energy-fields")
}
#[test]
fn m49_missing_durable_energy_field_keeps_corpse_and_boss_absent() -> TestResult {
    run("m49-energy-fields-missing")
}
#[test]
fn m49_fire_native_arrival_summons_once() -> TestResult {
    run("m49-fire")
}
#[test]
fn m49_ice_native_arrival_summons_once() -> TestResult {
    run("m49-ice")
}
#[test]
fn m49_earth_native_arrival_summons_once() -> TestResult {
    run("m49-earth")
}
#[test]
fn m49_missing_charge_keeps_corpse_and_boss_absent() -> TestResult {
    run("m49-fire-missing")
}

#[test]
fn m49_blocked_spawn_recovers_after_charge_revision_changes() -> TestResult {
    run("m49-fire-recover")
}
#[test]
fn m49_unknown_realization_ack_never_duplicates_the_actor() -> TestResult {
    run("m49-fire-ack")
}
#[test]
fn m49_arrival_retires_real_corpse_loot_with_the_root() -> TestResult {
    run("m49-fire-loot")
}

#[test]
fn m49_current_arrival_recovers_after_lost_projection_response() -> TestResult {
    run("m49-fire-response")
}
