//! Fury Gates and Nightmare Isles returns: the selectors the real native map carries
//! reach the production step gate (TibiaWiki Fury Gates Mini World Change 76237@1099676,
//! Nightmare Isles Mini World Change 76248@891482: the exit returns to the active city).
use super::TestResult;
use crate::foundation::MovementLocalPosition;
use oteryn_world_bundle::{bundle, sector};

/// The `(action, unique)` of the single source item `item` at `(x, y)` in a real B3 region.
fn native_selector(
    bytes: &[u8],
    (x, y): (u16, u16),
    item: u64,
) -> TestResult<(Option<u16>, Option<u16>)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/placements/index.json"),
    )?)?;
    let region = oteryn_world_bundle_compiler::b3::decode_region(
        bytes,
        bundle::TILE_LIMITS,
        &mut sector::Budget {
            tiles: 65536,
            entries: 1_000_000,
        },
    )?;
    let mut found = region
        .sectors
        .into_iter()
        .flat_map(|(_, _, tiles)| tiles)
        .filter(|tile| (tile.x, tile.y) == (x, y))
        .flat_map(|tile| tile.items)
        .filter(|entry| {
            index["palette"][entry.palette as usize]["source_item_id"].as_u64() == Some(item)
        })
        .map(|entry| (entry.attrs.action, entry.attrs.unique));
    let selector = found.next().ok_or("native return item")?;
    assert!(found.next().is_none(), "one native return item");
    Ok(selector)
}

#[test]
fn worldchg_fury_and_nightmare_real_map_returns_reach_the_selected_city() -> TestResult {
    let fury = native_selector(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z13-x130-y124.b3"
        )),
        (33290, 31785),
        776,
    )?;
    let nightmare = native_selector(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z08-x130-y127.b3"
        )),
        (33498, 32613),
        10840,
    )?;
    assert_eq!(fury, (Some(9715), None), "Fury Dungeon archway");
    assert_eq!(nightmare, (None, Some(35020)), "Nightmare Isle exit");
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("worldchg-real-returns", || {
        super::daily_world_gates::scenario(
            fury.0,
            fury.1,
            "fury_gates/thais",
            true,
            MovementLocalPosition {
                x: 32265,
                y: 32164,
                floor: 7,
            },
            false,
        )?;
        super::daily_world_gates::scenario(
            nightmare.0,
            nightmare.1,
            "nightmare_isle/darashia-north",
            true,
            MovementLocalPosition {
                x: 33215,
                y: 32273,
                floor: 7,
            },
            false,
        )?;
        eprintln!("POSTGRES_CASE_COMPLETED worldchg_real_returns");
        Ok(())
    })
}
