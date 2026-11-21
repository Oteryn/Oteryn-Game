//! The Katana route starts at its accessible key cache, before the locked reward room.
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
                "/../../../content/world/placements/region-z09-x125-y125.b3"
            )),
            region: (9, 125, 125),
            bounds: (32177, 32132, 32179, 32133),
            start: (32178, 32132),
            expected: vec![
                ExpectedTile {
                    position: (32177, 32132),
                    ids: vec![355],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32178, 32132),
                    ids: vec![355],
                    counted_entry: None,
                },
            ],
        },
    )
}
