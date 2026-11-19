//! Visit the retained infernal arrival approach without replacing its source portal.
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
                "/../../../content/world/placements/region-z08-x129-y126.b3"
            )),
            region: (8, 129, 126),
            bounds: (33275, 32390, 33278, 32391),
            start: (33277, 32390),
            expected: vec![
                ExpectedTile {
                    position: (33275, 32390),
                    ids: vec![409],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33276, 32390),
                    ids: vec![410],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33277, 32390),
                    ids: vec![409],
                    counted_entry: None,
                },
            ],
        },
    )
}
