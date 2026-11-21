//! Reach the retained Okolnir quest corpse without invoking its reward claim.
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
