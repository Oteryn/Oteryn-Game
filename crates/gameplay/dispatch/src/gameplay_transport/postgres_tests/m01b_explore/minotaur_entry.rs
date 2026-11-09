//! Find the retained deep Minotaur Hell room without claiming corpse rewards.
use super::*;

#[test]
fn m01b_minotaur_native_hell_entry_credits_once_and_replays() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run(
        "m01b-minotaur-hell-entry",
        move || {
            super::super::quest_world_callbacks::scenario(
                "m01b/minotaur-hell/s1/minohell1",
                Position {
                    x: 32131,
                    y: 32066,
                    floor: -12,
                },
                false,
            )
        },
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_minotaur_hell_entry duration={:?} success={}",
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
                "/../../../content/world/placements/region-z12-x125-y125.b3"
            )),
            region: (12, 125, 125),
            bounds: (32131, 32066, 32133, 32067),
            start: (32132, 32066),
            expected: vec![
                ExpectedTile {
                    position: (32131, 32066),
                    ids: vec![351],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32132, 32066),
                    ids: vec![351],
                    counted_entry: None,
                },
            ],
        },
    )
}
