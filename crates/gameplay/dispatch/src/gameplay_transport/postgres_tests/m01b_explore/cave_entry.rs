//! Enter the retained bear-cave approach before the Dragon Corpse shovel route.
use super::*;

#[test]
fn m01b_dragon_corpse_native_cave_entry_credits_once_and_replays() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run(
        "m01b-dragon-corpse-entry",
        move || {
            super::super::quest_world_callbacks::scenario(
                "m01b/dragon-corpse/s1/entry",
                Position {
                    x: 32147,
                    y: 32207,
                    floor: -7,
                },
                false,
            )
        },
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_dragon_corpse_entry duration={:?} success={}",
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
                "/../../../content/world/placements/region-z07-x125-y125.b3"
            )),
            region: (7, 125, 125),
            bounds: (32147, 32207, 32149, 32208),
            start: (32148, 32207),
            expected: vec![
                ExpectedTile {
                    position: (32147, 32207),
                    ids: vec![4409],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32148, 32207),
                    ids: vec![4406],
                    counted_entry: None,
                },
            ],
        },
    )
}
