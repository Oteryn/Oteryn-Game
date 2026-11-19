//! Native visits beside retained rewards; stepping never invokes the reward claim.
use super::*;





pub(in super::super) fn native_case(label: &'static str, name: &'static str, x: u16, y: u16, floor: i8) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e_fixtures::scenario_thread::run(label, move || {
        super::super::quest_world_callbacks_fixtures::scenario(name, Position { x, y, floor }, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {label} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(in super::super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::super::l6_world_pickups_fixtures::{ExpectedTile, NativeCrop, native_crop};
    let crop = match name {
        "m01b/fishing-box/s1/fishing_box" => NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z07-x125-y122.b3"
            )),
            region: (7, 125, 122),
            bounds: (32015, 31409, 32018, 31410),
            start: (32017, 31409),
            expected: vec![
                ExpectedTile {
                    position: (32015, 31409),
                    ids: vec![799, 2473],
                    counted_entry: None,
                },
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
            bounds: (33197, 31923, 33200, 31924),
            start: (33197, 31923),
            expected: vec![
                ExpectedTile {
                    position: (33199, 31923),
                    ids: vec![352, 2469],
                    counted_entry: None,
                },
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
