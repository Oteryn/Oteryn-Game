//! Exact carried-key absence selects the source post-boss fallback through real STEP.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn l6_tomb_carried_missing_3950_uses_source_fallback() -> TestResult {
    run(
        "l6-tomb-carried-3950",
        Position {
            x: 33231,
            y: 32705,
            floor: -8,
        },
    )
}
#[test]
fn l6_tomb_carried_missing_3951_uses_source_fallback() -> TestResult {
    run(
        "l6-tomb-carried-3951",
        Position {
            x: 33282,
            y: 32744,
            floor: -8,
        },
    )
}
#[test]
fn l6_tomb_carried_missing_3952_uses_source_fallback() -> TestResult {
    run(
        "l6-tomb-carried-3952",
        Position {
            x: 33250,
            y: 32832,
            floor: -8,
        },
    )
}

#[test]
fn l6_tomb_carried_present_3950_debits_exact_key() -> TestResult {
    run(
        "l6-tomb-carried-present-3950",
        Position {
            x: 33182,
            y: 32714,
            floor: -14,
        },
    )
}
#[test]
fn l6_tomb_carried_present_3951_debits_exact_key() -> TestResult {
    run(
        "l6-tomb-carried-present-3951",
        Position {
            x: 33174,
            y: 32934,
            floor: -15,
        },
    )
}
#[test]
fn l6_tomb_carried_present_3952_debits_exact_key() -> TestResult {
    run(
        "l6-tomb-carried-present-3952",
        Position {
            x: 33126,
            y: 32591,
            floor: -15,
        },
    )
}

fn run(name: &'static str, fallback: Position) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks_fixtures::stateless_scenario(
                name,
                Position {
                    x: 7,
                    y: 1,
                    floor: -7,
                },
                fallback,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "carried STEP scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
