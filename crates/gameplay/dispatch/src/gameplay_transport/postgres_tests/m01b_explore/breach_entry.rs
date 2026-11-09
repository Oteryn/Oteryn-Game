//! Visit the retained Zao/Farmine breach arrival without replacing its source portal.
use super::*;

#[test]
fn m01b_heart_native_zao_breach_requires_level_and_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e::scenario_thread::run("m01b-heart-zao-entry", move || {
            super::super::quest_world_callbacks::scenario(
                "m01b/heart-of-destruction/s1/entry",
                Position {
                    x: 32118,
                    y: 31355,
                    floor: -13,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_heart_zao_entry duration={:?} success={}",
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
                "/../../../content/world/placements/region-z13-x125-y122.b3"
            )),
            region: (13, 125, 122),
            bounds: (32117, 31355, 32120, 31356),
            start: (32119, 31355),
            expected: vec![
                ExpectedTile {
                    position: (32117, 31355),
                    ids: vec![23113],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32118, 31355),
                    ids: vec![23054],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32119, 31355),
                    ids: vec![23049],
                    counted_entry: None,
                },
            ],
        },
    )
}
