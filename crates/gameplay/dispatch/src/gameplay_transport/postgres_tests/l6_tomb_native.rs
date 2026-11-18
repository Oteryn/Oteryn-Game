//! Literal native portal room proofs; no synthetic UID or hidden raw destination.
use super::l6_tomb_native_fixtures::{run};

use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn l6_tomb_native_missing_3950_uses_source_branch() -> TestResult {
    run(
        "l6-tomb-native-missing-3950",
        Position {
            x: 33174,
            y: 32694,
            floor: -14,
        },
        Position {
            x: 33231,
            y: 32705,
            floor: -8,
        },
    )
}
#[test]
fn l6_tomb_native_present_3950_uses_source_branch() -> TestResult {
    super::l6_tomb_native_fixtures::present_3950()
}
#[test]
fn l6_tomb_native_missing_3951_uses_source_branch() -> TestResult {
    run(
        "l6-tomb-native-missing-3951",
        Position {
            x: 33396,
            y: 32852,
            floor: -14,
        },
        Position {
            x: 33282,
            y: 32744,
            floor: -8,
        },
    )
}
#[test]
fn l6_tomb_native_present_3951_uses_source_branch() -> TestResult {
    super::l6_tomb_native_fixtures::present_3951()
}
#[test]
fn l6_tomb_native_missing_3952_uses_source_branch() -> TestResult {
    run(
        "l6-tomb-native-missing-3952",
        Position {
            x: 33191,
            y: 32959,
            floor: -15,
        },
        Position {
            x: 33250,
            y: 32832,
            floor: -8,
        },
    )
}
#[test]
fn l6_tomb_native_present_3952_uses_source_branch() -> TestResult {
    super::l6_tomb_native_fixtures::present_3952()
}
