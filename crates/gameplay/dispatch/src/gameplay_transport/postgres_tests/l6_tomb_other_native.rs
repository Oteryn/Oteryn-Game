//! Remaining four literal Tomb portal rooms use the shared native branch proof.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn l6_tomb_other_native_missing_3953_uses_source_branch() -> TestResult {
    super::l6_tomb_native::run(
        "l6-tomb-native-missing-3953",
        Position {
            x: 33146,
            y: 32668,
            floor: -15,
        },
        Position {
            x: 33025,
            y: 32868,
            floor: -8,
        },
    )
}
#[test]
fn l6_tomb_other_native_present_3953_uses_source_branch() -> TestResult {
    super::l6_tomb_native::run(
        "l6-tomb-native-present-3953",
        Position {
            x: 33146,
            y: 32668,
            floor: -15,
        },
        Position {
            x: 33145,
            y: 32665,
            floor: -15,
        },
    )
}
#[test]
fn l6_tomb_other_native_missing_3954_uses_source_branch() -> TestResult {
    super::l6_tomb_native::run(
        "l6-tomb-native-missing-3954",
        Position {
            x: 33042,
            y: 32777,
            floor: -14,
        },
        Position {
            x: 33133,
            y: 32642,
            floor: -8,
        },
    )
}
#[test]
fn l6_tomb_other_native_present_3954_uses_source_branch() -> TestResult {
    super::l6_tomb_native::run(
        "l6-tomb-native-present-3954",
        Position {
            x: 33042,
            y: 32777,
            floor: -14,
        },
        Position {
            x: 33041,
            y: 32774,
            floor: -14,
        },
    )
}
#[test]
fn l6_tomb_other_native_missing_3955_uses_source_branch() -> TestResult {
    super::l6_tomb_native::run(
        "l6-tomb-native-missing-3955",
        Position {
            x: 33350,
            y: 32830,
            floor: -14,
        },
        Position {
            x: 33131,
            y: 32566,
            floor: -8,
        },
    )
}
#[test]
fn l6_tomb_other_native_present_3955_uses_source_branch() -> TestResult {
    super::l6_tomb_native::run(
        "l6-tomb-native-present-3955",
        Position {
            x: 33350,
            y: 32830,
            floor: -14,
        },
        Position {
            x: 33349,
            y: 32827,
            floor: -14,
        },
    )
}
#[test]
fn l6_tomb_other_native_missing_3956_uses_source_branch() -> TestResult {
    super::l6_tomb_native::run(
        "l6-tomb-native-missing-3956",
        Position {
            x: 33187,
            y: 33015,
            floor: -14,
        },
        Position {
            x: 33206,
            y: 32592,
            floor: -8,
        },
    )
}
#[test]
fn l6_tomb_other_native_present_3956_uses_source_branch() -> TestResult {
    super::l6_tomb_native::run(
        "l6-tomb-native-present-3956",
        Position {
            x: 33187,
            y: 33015,
            floor: -14,
        },
        Position {
            x: 33186,
            y: 33012,
            floor: -14,
        },
    )
}
