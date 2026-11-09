//! Native visits beside retained rewards; stepping never invokes the reward claim.
use super::*;

#[test]
fn m01b_treasure_fishing_box_native_entry_requires_current_premium() -> TestResult {
    native_case(
        "m01b-fishing-box-entry",
        "m01b/fishing-box/s1/fishing_box",
        32016,
        31409,
        -7,
    )
}

#[test]
fn m01b_treasure_berserker_native_entry_requires_current_premium() -> TestResult {
    native_case(
        "m01b-berserker-entry",
        "m01b/berserker-treasure/s1/white_pearls",
        33198,
        31923,
        -11,
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
        "m01b/fishing-box/s1/fishing_box" => NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z07-x125-y122.b3"
            )),
            region: (7, 125, 122),
            bounds: (32016, 31409, 32018, 31410),
            start: (32017, 31409),
            expected: vec![
                ExpectedTile {
                    position: (32016, 31409),
                    ids: vec![6587],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32017, 31409),
                    ids: vec![6580],
                    counted_entry: None,
                },
            ],
        },
        "m01b/berserker-treasure/s1/white_pearls" => NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z11-x129-y124.b3"
            )),
            region: (11, 129, 124),
            bounds: (33197, 31923, 33199, 31924),
            start: (33197, 31923),
            expected: vec![
                ExpectedTile {
                    position: (33198, 31923),
                    ids: vec![353],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33197, 31923),
                    ids: vec![353],
                    counted_entry: None,
                },
            ],
        },
        _ => return Err("unbound M01b treasure entry".into()),
    };
    native_crop(native, crop)
}
