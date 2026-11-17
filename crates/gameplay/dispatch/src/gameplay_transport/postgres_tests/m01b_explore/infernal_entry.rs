//! Visit the retained infernal arrival approach without replacing its source portal.
use super::*;

#[test]
fn m01b_ferumbras_native_arrival_requires_level_and_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e::scenario_thread::run("m01b-ferumbras-entry", move || {
            super::super::quest_world_callbacks::scenario(
                "m01b/ferumbras-ascension/s1/entry",
                Position {
                    x: 33276,
                    y: 32390,
                    floor: -8,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_ferumbras_entry duration={:?} success={}",
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
