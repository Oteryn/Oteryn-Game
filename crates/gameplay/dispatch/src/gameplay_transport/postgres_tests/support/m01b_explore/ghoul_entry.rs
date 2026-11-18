//! Reach the outer Ghoul Room passage before opening its final key door.
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
                "/../../../content/world/placements/region-z14-x126-y125.b3"
            )),
            region: (14, 126, 125),
            bounds: (32507, 32175, 32509, 32176),
            start: (32508, 32175),
            expected: vec![
                ExpectedTile {
                    position: (32507, 32175),
                    ids: vec![409],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32508, 32175),
                    ids: vec![409],
                    counted_entry: None,
                },
            ],
        },
    )
}
