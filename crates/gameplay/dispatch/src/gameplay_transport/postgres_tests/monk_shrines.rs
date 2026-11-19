//! The production callback content, real USE protocol, and the durable quest owner.

use super::TestResult;

use oteryn_gameplay_quests::quest::{
    monk_shrines,
    world_callbacks::{self, Position},
};

#[test]
fn monk_shrine_selectors_match_the_real_world_source_tiles() -> TestResult {
    let root = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/placements"
    ));
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("index.json"))?)?;
    let palette = index["palette"].as_array().ok_or("palette")?;
    for callback in world_callbacks::load()?
        .iter()
        .filter(|c| c.quest == monk_shrines::QUEST && c.monk_reward.is_some())
    {
        let p = callback.selector.position.ok_or("position")?;
        let name = format!(
            "region-z{:02}-x{:03}-y{:03}.b3",
            -p.floor,
            p.x / 256,
            p.y / 256
        );
        let mut budget = oteryn_world_bundle::sector::Budget {
            tiles: 65536,
            entries: 65536 * 4096,
        };
        let region = oteryn_world_bundle_compiler::b3::decode_region(
            &std::fs::read(root.join(name))?,
            oteryn_world_bundle::sector::TileLimits {
                max_entries: 4096,
                max_text_bytes: 65536,
            },
            &mut budget,
        )?;
        let tile = region
            .sectors
            .iter()
            .flat_map(|(_, _, tiles)| tiles)
            .find(|t| t.x == p.x && t.y == p.y)
            .ok_or("source tile")?;
        assert_eq!(tile.house, 0, "{}", callback.key);
        assert!(
            tile.items.iter().any(|item| item.depth == 0
                && palette
                    .get(item.palette as usize)
                    .and_then(|row| row["source_item_id"].as_u64())
                    == callback.selector.appearance.map(u64::from)),
            "{} must resolve the actual source shrine, not an empty/wall tile",
            callback.key
        );
    }
    Ok(())
}

#[test]
fn monk_shrines_real_use_persists_once_and_checks_each_level() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("monk-shrines", || {
        super::quest_world_callbacks_fixtures::scenario(
            "monk-guidance",
            Position {
                x: 32454,
                y: 32292,
                floor: -7,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED monk_shrines_real_use_persists_once_and_checks_each_level duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn monk_shrine_full_backpack_keeps_entitlement_until_retry() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("monk-full-backpack", || {
        super::quest_world_callbacks_fixtures::scenario(
            "monk-guidance-full-backpack",
            Position {
                x: 32454,
                y: 32292,
                floor: -7,
            },
            false,
        )
    })
}

#[test]
fn monk_shrine_capacity_keeps_entitlement_until_retry() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("monk-capacity", || {
        super::quest_world_callbacks_fixtures::scenario(
            "monk-guidance-capacity",
            Position {
                x: 32454,
                y: 32292,
                floor: -7,
            },
            false,
        )
    })
}
