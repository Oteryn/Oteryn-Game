//! Enter the retained bear-cave approach before the Dragon Corpse shovel route.
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
                "/../../../content/world/placements/region-z07-x125-y125.b3"
            )),
            region: (7, 125, 125),
            bounds: (32147, 32207, 32149, 32208),
            start: (32148, 32207),
            expected: vec![
                ExpectedTile {
                    position: (32147, 32207),
                    ids: vec![4409],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32148, 32207),
                    ids: vec![4406],
                    counted_entry: None,
                },
            ],
        },
    )
}
