//! Literal USE against retained palace B3 tiles and the actual authored bindings.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

fn run(stage: &'static str, trigger: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            super::quest_world_callbacks_fixtures::scenario(stage, trigger, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Asura dispatch thread failed")?
        .map_err(Into::into)
}
#[test]
fn engenc_postgres_native_destroyer_chamber_admission() -> TestResult {
    run(
        "engenc-asura-native-s2",
        Position {
            x: 32959,
            y: 32675,
            floor: -3,
        },
    )
}
#[test]
fn engenc_postgres_native_fire_chamber_admission() -> TestResult {
    run(
        "engenc-asura-native-s3",
        Position {
            x: 32960,
            y: 32679,
            floor: -2,
        },
    )
}
#[test]
fn engenc_postgres_native_energy_chamber_admission() -> TestResult {
    run(
        "engenc-asura-native-s4",
        Position {
            x: 32948,
            y: 32679,
            floor: -2,
        },
    )
}
