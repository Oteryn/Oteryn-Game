//! Descend into the outer Bear Room basement before the retained stone and key door.
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
                "/../../../content/world/placements/region-z11-x125-y125.b3"
            )),
            region: (11, 125, 125),
            bounds: (32145, 32102, 32146, 32104),
            start: (32145, 32103),
            expected: vec![
                ExpectedTile {
                    position: (32145, 32102),
                    ids: vec![355],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32145, 32103),
                    ids: vec![354],
                    counted_entry: None,
                },
            ],
        },
    )
}
