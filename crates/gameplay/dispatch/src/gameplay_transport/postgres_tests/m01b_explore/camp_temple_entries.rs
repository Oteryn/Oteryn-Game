//! Entry visits at the retained Porgol camp and Rookgaard goblin temple.
use super::*;

#[test]
fn m01b_camp_temple_iron_helmet_native_camp_entry_credits_once_and_replays() -> TestResult {
    native_case(
        "m01b-iron-helmet-camp",
        "m01b/iron-helmet/s1/ironhelmet",
        32770,
        32225,
        -7,
    )
}

#[test]
fn m01b_camp_temple_goblin_native_temple_entry_credits_once_and_replays() -> TestResult {
    native_case(
        "m01b-goblin-temple",
        "m01b/goblin-temple/s1/goblintemple",
        31973,
        32210,
        -12,
    )
}

fn native_case(label: &'static str, name: &'static str, x: u16, y: u16, floor: i8) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run(label, move || {
        super::super::quest_world_callbacks::scenario(name, Position { x, y, floor }, false)
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
    let crop = match name {
        "m01b/iron-helmet/s1/ironhelmet" => NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z07-x128-y125.b3"
            )),
            region: (7, 128, 125),
            bounds: (32770, 32225, 32772, 32226),
            start: (32771, 32225),
            expected: vec![
                ExpectedTile {
                    position: (32770, 32225),
                    ids: vec![4516],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32771, 32225),
                    ids: vec![4515],
                    counted_entry: None,
                },
            ],
        },
        "m01b/goblin-temple/s1/goblintemple" => NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z12-x124-y125.b3"
            )),
            region: (12, 124, 125),
            bounds: (31973, 32210, 31975, 32211),
            start: (31974, 32210),
            expected: vec![
                ExpectedTile {
                    position: (31973, 32210),
                    ids: vec![502],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (31974, 32210),
                    ids: vec![501],
                    counted_entry: None,
                },
            ],
        },
        _ => return Err("unbound M01b camp/temple entry".into()),
    };
    native_crop(native, crop)
}
