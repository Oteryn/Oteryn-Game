//! Enter the retained Behemoth habitat, not Cyclopolis's surface marker.
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
