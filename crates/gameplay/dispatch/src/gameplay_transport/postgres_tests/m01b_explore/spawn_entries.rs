//! Enter the retained Behemoth habitat, not Cyclopolis's surface marker.
use super::*;

#[test]
fn m01b_behemoth_native_floor_entry_requires_level_eighty_and_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run(
        "m01b-behemoth-floor-entry",
        move || {
            super::super::quest_world_callbacks::scenario(
                "m01b/behemoth/s1/entry",
                Position {
                    x: 33235,
                    y: 31695,
                    floor: -12,
                },
                false,
            )
        },
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_behemoth_floor_entry duration={:?} success={}",
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
                "/../../../content/world/placements/region-z12-x129-y123.b3"
            )),
            region: (12, 129, 123),
            bounds: (33235, 31695, 33237, 31696),
            start: (33236, 31695),
            expected: vec![
                ExpectedTile {
                    position: (33235, 31695),
                    ids: vec![499],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33236, 31695),
                    ids: vec![499],
                    counted_entry: None,
                },
            ],
        },
    )
}
