//! M20: laboratory stations consume every declared input atomically.
//! Scatter uses its unchanged native crop; Cradle uses a bounded mechanic scene.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn qready1_cradle_cauldron_three_inputs_one_antimutagen_atomic_and_repeatable() -> TestResult {
    run(
        "qready1-cradle-brew",
        Position {
            x: 33786,
            y: 32570,
            floor: -12,
        },
    )
}

#[test]
fn qready1_scatter_blender_five_inputs_one_flakes_atomic_and_repeatable() -> TestResult {
    run(
        "qready1-scatter-blend",
        Position {
            x: 32293,
            y: 32293,
            floor: -8,
        },
    )
}

fn run(name: &'static str, at: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks_fixtures::scenario(name, at, false)
                .map_err(|error| error.to_string())
        })?
        .join()
        .unwrap_or_else(|_| Err("M20 workbench worker panicked".into()));
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
