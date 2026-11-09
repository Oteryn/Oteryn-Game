//! Find the imperial passage key cache before the later locked-chamber stage.
use super::*;

#[test]
fn m01b_emperor_cookies_native_passage_entry_credits_once_and_replays() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e::scenario_thread::run(
        "m01b-emperor-cookies-entry",
        move || {
            super::super::quest_world_callbacks::scenario(
                "m01b/emperor-s-cookies/s1/id3800",
                Position {
                    x: 32605,
                    y: 31907,
                    floor: -3,
                },
                false,
            )
        },
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_emperor_cookies_entry duration={:?} success={}",
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
                "/../../../content/world/placements/region-z03-x127-y124.b3"
            )),
            region: (3, 127, 124),
            bounds: (32605, 31907, 32607, 31908),
            start: (32606, 31907),
            expected: vec![
                ExpectedTile {
                    position: (32605, 31907),
                    ids: vec![436],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (32606, 31907),
                    ids: vec![436],
                    counted_entry: None,
                },
            ],
        },
    )
}
