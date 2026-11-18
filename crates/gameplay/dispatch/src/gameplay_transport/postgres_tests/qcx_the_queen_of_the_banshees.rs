use super::{TestResult, kill_reward_e2e::scenario_thread, quest_world_callbacks};
use oteryn_gameplay_quests::quest::world_callbacks::{self, Position};

#[test]
fn final_room_forcefield_returns_to_ghostlands_surface() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }

    scenario_thread::run("qcx76-final-room-exit", || {
        let started = std::time::Instant::now();
        let name = "banshees-final-battle-exit";
        quest_world_callbacks::scenario(
            name,
            Position {
                x: 32219,
                y: 31913,
                floor: -15,
            },
            false,
        )?;
        eprintln!("qcx76 final room dispatch {:.2?}", started.elapsed());

        // The scenario checks the dispatched position against the loaded callback
        // and verifies its durable seal resets. Compare that same callback with
        // the independent Mapper Coords in /Spoiler, curid 34100, oldid 1057212.
        let callback = world_callbacks::load()?
            .into_iter()
            .find(|callback| callback.key.ends_with(name))
            .ok_or("final battle exit callback missing")?;
        assert_eq!(
            callback.teleport,
            Some(Position {
                x: 125 * 256 + 202,
                y: 124 * 256 + 102,
                floor: -7,
            })
        );
        assert_source_tag((32219, 31913, 15), 1949, Some(35019), None)
    })
}

#[test]
fn final_room_chests_mint_the_six_global_rewards_once() -> TestResult {
    use super::quest_catalogue_chests::{CatalogueChest, execute};

    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }

    // Quest page curid 3216, oldid 1038207. Crystal chest.lua:1000-1041
    // supplies the selectors, item identifiers and the existing charge operand.
    let chests = [
        ("stone_skin_amulet", 32212, 31896, 3081, 1),
        ("stealth_ring", 32226, 31896, 3049, 1),
        ("tower_shield", 32212, 31910, 3428, 1),
        ("giant_sword", 32226, 31910, 3281, 1),
        ("boots_of_haste", 32218, 31912, 3079, 1),
        ("platinum_coin", 32220, 31912, 3035, 100),
    ]
    .into_iter()
    .map(|(reward, x, y, id, quantity)| {
        let root = format!("oteryn:item.tibia.i{id}");
        let charges = if id == 3081 {
            vec![(root.clone(), 1)]
        } else {
            Vec::new()
        };
        CatalogueChest {
            claim: format!("oteryn:reward-claim.quest.u7_2.the_queen_of_the_banshees.reward.{reward}"),
            position: (x, y, -15),
            unique: 0,
            servable: true,
            appearance: 2472,
            root,
            quantity,
            contents: vec![],
            texts: vec![],
            key_number: None,
            achievement: false,
            charges,
            fluids: vec![],
        }
    })
    .collect();
    execute(chests)
}

#[test]
fn first_seal_return_and_second_seal_flame_bind_to_native_tiles() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }

    scenario_thread::run("qcx76-seal-teleports", || {
        for (name, x, y, floor) in [
            ("banshees-first-seal-return", 32266, 31898, -12),
            ("banshees-second-seal", 32171, 31853, -15),
        ] {
            let started = std::time::Instant::now();
            quest_world_callbacks::scenario(name, Position { x, y, floor }, false)?;
            eprintln!("qcx76 {name} dispatch {:.2?}", started.elapsed());
        }
        for x in 32265..=32267 {
            assert_source_tag((x, 31898, 12), 353, None, Some(25019))?;
        }
        assert_source_tag((32171, 31853, 15), 1959, Some(35014), None)
    })
}

fn assert_source_tag(
    (x, y, z): (u16, u16, u8),
    appearance: u16,
    unique: Option<u16>,
    action: Option<u16>,
) -> TestResult {
    use oteryn_world_bundle::{bundle, sector::Budget};
    use oteryn_world_bundle_compiler::b3;

    let placements = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/placements"
    ));
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(placements.join("index.json"))?)?;
    let raw = std::fs::read(placements.join(format!(
        "region-z{z:02}-x{:03}-y{:03}.b3",
        x / 256,
        y / 256
    )))?;
    let region = b3::decode_region(
        &raw,
        bundle::TILE_LIMITS,
        &mut Budget {
            tiles: 65536,
            entries: 1_000_000,
        },
    )?;
    let tile = region
        .sectors
        .iter()
        .flat_map(|(_, _, tiles)| tiles)
        .find(|tile| tile.x == x && tile.y == y)
        .ok_or("source tile")?;
    assert_eq!(tile.house, 0);
    assert!(tile.items.iter().any(|item| {
        item.depth == 0
            && index["palette"][item.palette as usize]["source_item_id"] == appearance
            && item.attrs.unique == unique
            && item.attrs.action == action
    }), "source selector at ({x}, {y}, {z})");
    Ok(())
}
