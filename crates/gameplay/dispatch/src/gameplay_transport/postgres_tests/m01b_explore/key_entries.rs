//! The Katana route starts at its accessible key cache, before the locked reward room.
use super::*;

#[test]
fn m01b_katana_native_key_route_entry_credits_once_without_claiming_key() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e::scenario_thread::run("m01b-katana-key-entry", move || {
            super::super::quest_world_callbacks::scenario(
                "m01b/katana/s1/id4603",
                Position {
                    x: 32177,
                    y: 32132,
                    floor: -9,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_katana_key_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop};
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
