//! W05d source seasonal NPC points qualified against shipped native map ground.
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use oteryn_world_bundle_compiler::bundle::{BuildClass, Identity};
use oteryn_world_bundle_compiler::compile::{Input, realize_npcs, realize_spawns};
use oteryn_world_bundle_compiler::project::{self, Families};
use oteryn_world_bundle_compiler::resolve::Registry;
use serde_json::Value;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn shards(root: &Path, directory: &str, prefix: &str) -> TestResult<Vec<Vec<u8>>> {
    let mut paths = fs::read_dir(root.join(directory))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".json"))
    });
    paths.sort();
    paths
        .into_iter()
        .map(|path| fs::read(path).map_err(Into::into))
        .collect()
}

#[test]
fn seasonal_npcs_twenty_calendar_points_brutus_and_machine_pads_bind_real_ground() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let points = [
        ("oteryn:npc.brutus_the_baker", (33303, 32064, 7)),
        ("oteryn:npc.balduin_the_baker", (33192, 32401, 7)),
        ("oteryn:npc.benny_the_baker", (32345, 31819, 7)),
        ("oteryn:npc.biff_the_baker", (32348, 32272, 7)),
        ("oteryn:npc.billy_the_baker", (32375, 32798, 7)),
        ("oteryn:npc.brian_the_baker", (33248, 31799, 7)),
        ("oteryn:npc.santa_claus", (32239, 31138, 7)),
        ("oteryn:npc.santa_claus", (32307, 31837, 8)),
        ("oteryn:npc.santa_claus", (32322, 32834, 7)),
        ("oteryn:npc.santa_claus", (32357, 32210, 7)),
        ("oteryn:npc.santa_claus", (32623, 32751, 7)),
        ("oteryn:npc.santa_claus", (32642, 31892, 9)),
        ("oteryn:npc.santa_claus", (32655, 31665, 8)),
        ("oteryn:npc.santa_claus", (32809, 31245, 7)),
        ("oteryn:npc.santa_claus", (32915, 32070, 9)),
        ("oteryn:npc.santa_claus", (32999, 31538, 10)),
        ("oteryn:npc.santa_claus", (33070, 32878, 6)),
        ("oteryn:npc.santa_claus", (33192, 31810, 7)),
        ("oteryn:npc.santa_claus", (33233, 32488, 7)),
        ("oteryn:npc.santa_claus", (33447, 31317, 8)),
        ("oteryn:npc.santa_claus", (33613, 31899, 6)),
        ("oteryn:npc.messenger_of_santa", (32240, 31138, 7)),
        ("oteryn:npc.messenger_of_santa", (32308, 31837, 8)),
        ("oteryn:npc.messenger_of_santa", (32323, 32834, 7)),
        ("oteryn:npc.messenger_of_santa", (32358, 32210, 7)),
        ("oteryn:npc.messenger_of_santa", (32624, 32751, 7)),
        ("oteryn:npc.messenger_of_santa", (32642, 31893, 9)),
        ("oteryn:npc.messenger_of_santa", (32655, 31666, 8)),
        ("oteryn:npc.messenger_of_santa", (32810, 31245, 7)),
        ("oteryn:npc.messenger_of_santa", (32916, 32070, 9)),
        ("oteryn:npc.messenger_of_santa", (33000, 31538, 10)),
        ("oteryn:npc.messenger_of_santa", (33071, 32878, 6)),
        ("oteryn:npc.messenger_of_santa", (33193, 31810, 7)),
        ("oteryn:npc.messenger_of_santa", (33234, 32488, 7)),
        ("oteryn:npc.messenger_of_santa", (33448, 31317, 8)),
        ("oteryn:npc.messenger_of_santa", (33614, 31899, 6)),
    ];
    let mut families = Families::default();
    for (ordinal, destination) in [
        (32346, 31822, 7),
        (32346, 31823, 7),
        (32350, 32275, 7),
        (32350, 32276, 7),
        (33251, 31802, 7),
        (33251, 31803, 7),
        (33191, 32397, 7),
        (33191, 32398, 7),
        (32377, 32801, 7),
        (32377, 32802, 7),
    ]
    .into_iter()
    .enumerate()
    {
        families.routes.push(project::TravelRoute {
            service: "oteryn:service.cake_machine_validation".into(),
            route: format!("pad_{ordinal}"),
            destination,
        });
    }

    // Retained source raid point; validate its real source ground and route
    // through the same native compiler used by the seasonal NPC placements.
    families.routes.push(project::TravelRoute {
        service: "oteryn:service.halloween_hare_validation".into(),
        route: "source_landing".into(),
        destination: (32624, 32018, 7),
    });
    for bytes in shards(&root, "content/npcs/definitions", "npcs-")? {
        families.add_npcs(&bytes)?;
    }
    for bytes in shards(&root, "content/dialogues/definitions", "dialogues-")? {
        families.add_dialogues(&bytes)?;
    }
    // Messenger uses its real thirteen-item Trade service. Loading NPCs alone
    // cannot qualify a production placement with a declared service reference.
    for bytes in shards(&root, "content/services/trade", "trade-")? {
        families.add_services(&bytes)?;
    }
    // The calendar runtime uses these exact source positions. Feed the production
    // compiler their authored placements, preserving every real B3 collision fact.
    families.npc_placements = points
        .iter()
        .enumerate()
        .map(|(i, (npc, cell))| project::NpcPlacement {
            key: format!("oteryn:npc_placement.seasonal_{i}"),
            npc: (*npc).into(),
            cell: *cell,
            direction: oteryn_world_bundle_compiler::spawn::Direction::South,
        })
        .collect();
    let regions = points
        .iter()
        .map(|(_, (x, y, z))| {
            format!(
                "content/world/placements/region-z{z:02}-x{:03}-y{:03}.b3",
                x / 256,
                y / 256,
            )
        })
        .chain(std::iter::once(
            "content/world/placements/region-z07-x127-y125.b3".to_owned(),
        ))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|p| fs::read(root.join(p)))
        .collect::<Result<Vec<_>, _>>()?;
    let index: Value =
        serde_json::from_slice(&fs::read(root.join("content/world/placements/index.json"))?)?;
    let mut registry = Registry::default();
    for bytes in shards(&root, "content/items/definitions", "items-")? {
        registry.add_items(&bytes)?;
    }
    registry.add_item_profiles(&fs::read(
        root.join("content/items/definitions/spell-native-profiles.json"),
    )?)?;
    for (family, directory, prefix) in [
        ("Terrain", "content/world/terrain", "terrain-"),
        ("WorldObject", "content/world/objects", "objects-"),
    ] {
        for bytes in shards(&root, directory, prefix)? {
            registry.add_catalogue(family, &bytes)?;
        }
    }
    let palette = index["palette"]
        .as_array()
        .ok_or("palette")?
        .iter()
        .map(|entry| {
            let key = entry["key"].as_str().ok_or("palette key")?.to_owned();
            if entry["provisional"] == true {
                registry.add_provisional(key.clone());
            }
            Ok(key)
        })
        .collect::<TestResult<Vec<_>>>()?;
    registry.seal()?;
    let worlds = shards(&root, "content/world/worlds", "worlds-")?;
    let input = Input {
        regions: &regions,
        palette: &palette,
        identity: Identity::default(),
        world: project::world_extent(worlds.first().ok_or("World shard")?)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &families,
    };
    let (spawns, _) = realize_spawns(&input, &registry)?;
    let (npcs, report) = realize_npcs(&input, &registry, &spawns)?;
    assert!(report.held.is_empty(), "{report:#?}");
    assert!(report.routes_held.is_empty(), "{report:#?}");
    assert_eq!(report.routes, 11);
    assert_eq!(npcs.placement_count(), 36);
    assert_eq!(npcs.npcs.len(), 8);
    Ok(())
}
