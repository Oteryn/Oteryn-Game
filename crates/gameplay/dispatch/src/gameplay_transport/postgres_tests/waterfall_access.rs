//! Waterfall's source teleports and splash run through the real STEP dispatcher.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn waterfall_source_selectors_match_native_map() -> TestResult {
    let root = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/placements"
    ));
    let mut count = 0;
    for callback in oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .filter(|c| c.quest == "oteryn:quest.waterfall_quest")
    {
        let p = callback.selector.position.ok_or("source position")?;
        let bytes = std::fs::read(root.join(format!(
            "region-z{:02}-x{:03}-y{:03}.b3",
            -p.floor,
            p.x / 256,
            p.y / 256
        )))?;
        let region = oteryn_world_bundle_compiler::b3::decode_region(
            &bytes,
            oteryn_world_bundle::bundle::TILE_LIMITS,
            &mut oteryn_world_bundle::sector::Budget {
                tiles: 65536,
                entries: 1_000_000,
            },
        )?;
        let tile = region
            .sectors
            .iter()
            .flat_map(|(_, _, tiles)| tiles)
            .find(|tile| tile.x == p.x && tile.y == p.y)
            .ok_or("source tile")?;
        assert_eq!(tile.house, 0);
        assert!(
            tile.items
                .iter()
                .any(|item| item.depth == 0 && item.attrs.action == Some(50022)),
            "{} has no native source action50022",
            callback.key
        );
        count += 1;
    }
    assert_eq!(count, 4);
    Ok(())
}

#[test]
fn waterfall_access_steps_preserve_quest_state() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .name("waterfall-access".into())
        .stack_size(256 << 20)
        .spawn(|| {
            for (name, from, to) in [
                ("waterfall-cliff", (32966, 32626, -5), (32968, 32626, -5)),
                ("waterfall-splash", (32968, 32630, -7), (32968, 32630, -7)),
                (
                    "waterfall-cave-entry",
                    (32968, 32629, -7),
                    (32968, 32631, -8),
                ),
                (
                    "waterfall-cave-exit",
                    (32967, 32630, -8),
                    (32971, 32620, -8),
                ),
            ] {
                let position = |(x, y, floor)| Position { x, y, floor };
                super::quest_world_callbacks::stateless_scenario(
                    name,
                    position(from),
                    position(to),
                )
                .map_err(|error| error.to_string())?;
            }
            Ok::<(), String>(())
        })?
        .join()
        .map_err(|_| "waterfall access scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED waterfall_access Waterfall access duration={:?}",
        started.elapsed()
    );
    Ok(())
}
