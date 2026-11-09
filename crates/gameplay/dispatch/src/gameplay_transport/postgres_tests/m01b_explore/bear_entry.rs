//! Descend into the outer Bear Room basement before the retained stone and key door.
use super::*;

#[test]
fn m01b_bear_native_basement_entry_credits_once_before_key_and_stone() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run(
        "m01b-bear-basement-entry",
        move || {
            super::super::quest_world_callbacks::scenario(
                "m01b/bear-room/s1/entry",
                Position {
                    x: 32145,
                    y: 32102,
                    floor: -11,
                },
                false,
            )
        },
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_bear_basement_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop};
    native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z11-x125-y125.b3"
            )),
            region: (11, 125, 125),
            bounds: (32145, 32102, 32146, 32104),
            start: (32145, 32103),
            expected: vec![
                ExpectedTile {
                    position: (32145, 32102),
                    ids: vec![355],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32145, 32103),
                    ids: vec![354],
                    counted_entry: None,
                },
            ],
        },
    )
}
