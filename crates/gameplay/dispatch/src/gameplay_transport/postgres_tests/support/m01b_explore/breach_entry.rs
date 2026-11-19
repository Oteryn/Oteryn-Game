//! Visit the retained Zao/Farmine breach arrival without replacing its source portal.
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
                "/../../../content/world/placements/region-z13-x125-y122.b3"
            )),
            region: (13, 125, 122),
            bounds: (32117, 31355, 32120, 31356),
            start: (32119, 31355),
            expected: vec![
                ExpectedTile {
                    position: (32117, 31355),
                    ids: vec![23113],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32118, 31355),
                    ids: vec![23054],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32119, 31355),
                    ids: vec![23049],
                    counted_entry: None,
                },
            ],
        },
    )
}
