//! Clear retained crypt approaches; entry observes the room without claiming its chest.
use super::*;

#[test]
fn m01b_crypt_inukaya_native_entry_requires_current_premium() -> TestResult {
    native_case(
        "m01b-inukaya-entry",
        "m01b/inukaya/s1/inukaya",
        32484,
        31109,
        -8,
    )
}
#[test]
fn m01b_crypt_medusa_native_entry_requires_level_and_current_premium() -> TestResult {
    native_case(
        "m01b-medusa-entry",
        "m01b/medusa-shield/s1/the_medusa_quest",
        33050,
        32398,
        -10,
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
        "m01b/inukaya/s1/inukaya" => NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z08-x126-y121.b3"
            )),
            region: (8, 126, 121),
            bounds: (32484, 31109, 32486, 31110),
            start: (32485, 31109),
            expected: vec![
                ExpectedTile {
                    position: (32484, 31109),
                    ids: vec![6684],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32485, 31109),
                    ids: vec![6685],
                    counted_entry: None,
                },
            ],
        },
        "m01b/medusa-shield/s1/the_medusa_quest" => NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z10-x129-y126.b3"
            )),
            region: (10, 129, 126),
            bounds: (33050, 32398, 33052, 32399),
            start: (33051, 32398),
            expected: vec![
                ExpectedTile {
                    position: (33050, 32398),
                    ids: vec![410],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (33051, 32398),
                    ids: vec![410],
                    counted_entry: None,
                },
            ],
        },
        _ => return Err("unbound M01b crypt approach".into()),
    };
    native_crop(native, crop)
}
