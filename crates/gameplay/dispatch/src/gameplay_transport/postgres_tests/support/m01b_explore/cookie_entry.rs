//! Find the imperial passage key cache before the later locked-chamber stage.
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
                "/../../../content/world/placements/region-z03-x127-y124.b3"
            )),
            region: (3, 127, 124),
            bounds: (32605, 31907, 32607, 31908),
            start: (32606, 31907),
            expected: vec![
                ExpectedTile {
                    position: (32605, 31907),
                    ids: vec![436],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32606, 31907),
                    ids: vec![436],
                    counted_entry: None,
                },
            ],
        },
    )
}
