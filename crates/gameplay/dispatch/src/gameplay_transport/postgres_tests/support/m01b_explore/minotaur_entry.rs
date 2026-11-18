//! Find the retained deep Minotaur Hell room without claiming corpse rewards.
use super::*;



pub(in super::super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::super::l6_world_pickups_fixtures::{ExpectedTile, NativeCrop, native_crop};
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
