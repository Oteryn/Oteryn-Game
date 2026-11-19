//! Literal native portal room proofs; no synthetic UID or hidden raw destination.
use super::l6_tomb_native_fixtures::{run};

use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn l6_tomb_native_missing_3950_uses_source_branch() -> TestResult {
    run(
        "l6-tomb-native-missing-3950",
        Position {
            x: 33183,
            y: 32717,
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
    run(
        "l6-tomb-native-present-3950",
        Position {
            x: 33183,
            y: 32717,
            floor: -14,
        },
        Position {
            x: 33182,
            y: 32714,
            floor: -14,
        },
    )
}
#[test]
fn l6_tomb_native_missing_3951_uses_source_branch() -> TestResult {
    run(
        "l6-tomb-native-missing-3951",
        Position {
            x: 33175,
            y: 32937,
            floor: -15,
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
    run(
        "l6-tomb-native-present-3951",
        Position {
            x: 33175,
            y: 32937,
            floor: -15,
        },
        Position {
            x: 33174,
            y: 32934,
            floor: -15,
        },
    )
}
#[test]
fn l6_tomb_native_missing_3952_uses_source_branch() -> TestResult {
    run(
        "l6-tomb-native-missing-3952",
        Position {
            x: 33127,
            y: 32594,
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
    run(
        "l6-tomb-native-present-3952",
        Position {
            x: 33127,
            y: 32594,
            floor: -15,
        },
        Position {
            x: 33126,
            y: 32591,
            floor: -15,
        },
    )
}
