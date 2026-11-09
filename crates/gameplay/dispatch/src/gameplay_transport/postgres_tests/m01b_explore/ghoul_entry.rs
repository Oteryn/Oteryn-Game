//! Reach the outer Ghoul Room passage before opening its final key door.
use super::*;

#[test]
fn m01b_ghoul_native_outer_passage_credits_once_before_key_door() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run(
        "m01b-ghoul-outer-passage",
        move || {
            super::super::quest_world_callbacks::scenario(
                "m01b/ghoul-room/s1/entry",
                Position {
                    x: 32507,
                    y: 32175,
                    floor: -14,
                },
                false,
            )
        },
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_ghoul_outer_passage duration={:?} success={}",
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
