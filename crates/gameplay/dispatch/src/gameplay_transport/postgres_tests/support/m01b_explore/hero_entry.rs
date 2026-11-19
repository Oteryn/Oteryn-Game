//! Investigate the retained pipe-room approach before the later dungeon access door.
use super::*;



pub(in super::super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    super::super::l6_world_pickups_fixtures::native_crop(native, crop())
}

pub(in super::super) fn crop() -> super::super::l6_world_pickups_fixtures::NativeCrop {
    use super::super::l6_world_pickups_fixtures::{ExpectedTile, NativeCrop};
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
    }
}
