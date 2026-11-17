//! Actual STEP entry on explicit clear tiles beside retained Area markers.
use super::*;

#[test]
fn m01b_area_entries_dragon_tower_native_entry_credits_once_and_replays() -> TestResult {
    native_case(
        "m01b-dragon-tower-entry",
        "m01b/dragon-tower/s1/entry",
        33075,
        32169,
    )
}

#[test]
fn m01b_area_entries_berserk_potion_native_entry_requires_current_premium() -> TestResult {
    native_case(
        "m01b-berserk-potion-entry",
        "m01b/berserk-potion/s1/entry",
        32479,
        31179,
    )
}

fn native_case(label: &'static str, name: &'static str, x: u16, y: u16) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run(label, move || {
        super::super::quest_world_callbacks::scenario(name, Position { x, y, floor: -7 }, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {label} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop};
    let spec = match name {
        "m01b/dragon-tower/s1/entry" => NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z07-x129-y125.b3"
            )),
            region: (7, 129, 125),
            bounds: (33074, 32169, 33076, 32170),
            start: (33074, 32169),
            expected: vec![ExpectedTile {
                position: (33075, 32169),
                ids: vec![408],
                counted_entry: None,
            }],
        },
        "m01b/berserk-potion/s1/entry" => NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z07-x126-y121.b3"
            )),
            region: (7, 126, 121),
            bounds: (32479, 31179, 32481, 31180),
            start: (32480, 31179),
            expected: vec![
                ExpectedTile {
                    position: (32479, 31179),
                    ids: vec![6593],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32480, 31179),
                    ids: vec![6590],
                    counted_entry: None,
                },
            ],
        },
        _ => return Err("unbound M01b Area entry scene".into()),
    };
    native_crop(native, spec)
}
