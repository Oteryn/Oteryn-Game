//! Crystal's helmet altar consumes seven real floor pieces after the final wire drop.
use super::l6_floor_assembly_fixtures::{ALTAR};

use super::TestResult;

#[test]
fn l6_floor_assembly_final_drop_consumes_seven_and_creates_one_helmet() -> TestResult {
    run_case("l6-floor-assembly")
}

#[test]
fn l6_floor_ruby_preserves_helmet_and_starts_exact_deadline() -> TestResult {
    run_case("l6-floor-assembly-ruby")
}

#[test]
fn l6_floor_ruby_due_cycle_expires_same_physical_helmet() -> TestResult {
    run_case("l6-floor-assembly-ruby-cycle")
}

#[test]
fn l6_floor_ruby_due_cycle_carried_keeps_original_deadline() -> TestResult {
    run_case("l6-floor-assembly-ruby-cycle-carried")
}
#[test]
fn l6_floor_ruby_due_cycle_nested_keeps_original_deadline() -> TestResult {
    run_case("l6-floor-assembly-ruby-cycle-nested")
}
#[test]
fn l6_floor_ruby_due_cycle_equipped_advances_equipment_epoch() -> TestResult {
    run_case("l6-floor-assembly-ruby-cycle-equipped")
}
#[test]
fn l6_floor_ruby_due_cycle_future_is_not_early() -> TestResult {
    run_case("l6-floor-assembly-ruby-cycle-future")
}
#[test]
fn l6_floor_ruby_due_cycle_unhosted_equipment_is_refused() -> TestResult {
    run_case("l6-floor-assembly-ruby-cycle-unhosted")
}

#[test]
fn l6_floor_ruby_missing_target_keeps_input() -> TestResult {
    run_case("l6-floor-assembly-ruby-missing-target")
}

#[test]
fn l6_floor_ruby_stack_consumes_exactly_one() -> TestResult {
    run_case("l6-floor-assembly-ruby-stack")
}

#[test]
fn l6_floor_ruby_unbound_tile_keeps_input() -> TestResult {
    run_case("l6-floor-assembly-ruby-unbound")
}

#[test]
fn l6_floor_assembly_missing_part_keeps_every_other_item() -> TestResult {
    run_case("l6-floor-assembly-missing")
}

#[test]
fn l6_floor_assembly_unbound_tile_cannot_assemble() -> TestResult {
    run_case("l6-floor-assembly-unbound")
}

fn run_case(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks_fixtures::scenario(name, ALTAR, false)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "floor assembly scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
