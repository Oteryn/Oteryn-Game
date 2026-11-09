//! Investigate the retained pipe-room approach before the later dungeon access door.
use super::*;

#[test]
fn m01b_hero_native_voice_approach_requires_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e::scenario_thread::run("m01b-hero-voice-entry", move || {
            super::super::quest_world_callbacks::scenario(
                "m01b/hero-of-rathleton/s1/entry",
                Position {
                    x: 33533,
                    y: 31951,
                    floor: -14,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_hero_voice_entry duration={:?} success={}",
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
                "/../../../content/world/placements/region-z14-x130-y124.b3"
            )),
            region: (14, 130, 124),
            bounds: (33533, 31951, 33535, 31952),
            start: (33534, 31951),
            expected: vec![
                ExpectedTile {
                    position: (33533, 31951),
                    ids: vec![20545],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33534, 31951),
                    ids: vec![20533],
                    counted_entry: None,
                },
            ],
        },
    )
}
