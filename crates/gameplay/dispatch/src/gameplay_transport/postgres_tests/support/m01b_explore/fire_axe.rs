//! The Edron lair entry is approached from its clear western approach to the outer entry.
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
                "/../../../content/world/placements/region-z07-x129-y123.b3"
            )),
            region: (7, 129, 123),
            bounds: (33097, 31703, 33099, 31704),
            start: (33097, 31703),
            expected: vec![
                ExpectedTile {
                    position: (33098, 31703),
                    ids: vec![4408],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33097, 31703),
                    ids: vec![4406, 1103],
                    counted_entry: None,
                },
            ],
        },
    )
}
