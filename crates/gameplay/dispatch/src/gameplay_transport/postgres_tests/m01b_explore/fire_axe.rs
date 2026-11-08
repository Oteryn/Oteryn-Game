//! The Edron lair entry is approached from its walkable southern border.
use super::*;

#[test]
fn m01b_fire_axe_native_entry_keeps_level_sixty_and_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run("m01b-fire-axe-entry", || {
        super::super::quest_world_callbacks::scenario(
            "m01b/fire-axe/s1/entry",
            Position {
                x: 33098,
                y: 31701,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_fire_axe_entry duration={:?} success={}",
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
                "/../../../content/world/placements/region-z07-x129-y123.b3"
            )),
            region: (7, 129, 123),
            bounds: (33098, 31701, 33099, 31703),
            start: (33098, 31702),
            expected: vec![
                ExpectedTile {
                    position: (33098, 31701),
                    ids: vec![4409],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33098, 31702),
                    ids: vec![4407, 1855],
                    counted_entry: None,
                },
            ],
        },
    )
}
