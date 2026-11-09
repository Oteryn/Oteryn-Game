//! Reach the retained Okolnir quest corpse without invoking its reward claim.
use super::*;

#[test]
fn m01b_frost_dragon_native_lair_entry_requires_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e::scenario_thread::run("m01b-frost-dragon-entry", move || {
            super::super::quest_world_callbacks::scenario(
                "m01b/frost-dragon/s1/okolnir",
                Position {
                    x: 32146,
                    y: 31431,
                    floor: -8,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_frost_dragon_entry duration={:?} success={}",
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
                "/../../../content/world/placements/region-z08-x125-y122.b3"
            )),
            region: (8, 125, 122),
            bounds: (32146, 31430, 32148, 31432),
            start: (32147, 31431),
            expected: vec![
                ExpectedTile {
                    position: (32146, 31430),
                    ids: vec![4396, 2473],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32146, 31431),
                    ids: vec![4407],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32147, 31431),
                    ids: vec![4404],
                    counted_entry: None,
                },
            ],
        },
    )
}
