//! SUMMER: Mayor Pocaro and all 398 Radiant points on their admitted real ground.
use std::collections::{BTreeMap, BTreeSet};
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
fn summer_population_mayor_and_radiants_bind_to_real_map_cells() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // Crystal's summer and current maps agree on these populations. This is source-map
    // qualification, not independent proof of Global spawn coordinates or attack balance.
    let expected: BTreeMap<String, usize> = [
        ("acolyte", 51),
        ("inquisitor", 56),
        ("paragon", 84),
        ("templar", 64),
        ("warden", 78),
        ("zealot", 65),
    ]
    .into_iter()
    .map(|(name, count)| (format!("oteryn:creature.radiant_{name}"), count))
    .collect();
    let mut families = Families::default();
    for bytes in shards(&root, "content/creatures/definitions", "creatures-")? {
        families.add_creatures(&bytes)?;
    }
    for bytes in shards(&root, "content/world/spawns", "spawns-")? {
        families.add_spawns(&bytes)?;
    }
    // Keep the authored points and the real region bytes; no synthetic ground or resolver.
    for source in &mut families.spawns {
        source
            .points
            .retain(|point| expected.contains_key(&point.creature));
    }
    families.spawns.retain(|source| !source.points.is_empty());
    let mut authored = BTreeMap::new();
    let mut regions = BTreeSet::new();
    for source in &families.spawns {
        for point in &source.points {
            *authored.entry(point.creature.clone()).or_insert(0) += 1;
            let (x, y, z) = point.cell;
            regions.insert(format!(
                "content/world/placements/region-z{z:02}-x{:03}-y{:03}.b3",
                x / 256,
                y / 256
            ));
        }
    }
    assert_eq!(authored, expected);
    let regions = regions
        .into_iter()
        .map(|path| fs::read(root.join(path)))
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
    let (table, report) = realize_spawns(&input, &registry)?;
    // Decision 21 admits the exact appearance-only ground through Terrain.
    assert!(report.dropped.is_empty(), "{report:#?}");
    let mut realized = BTreeMap::new();
    for source in &table.sources {
        for point in &source.points {
            *realized
                .entry(table.creatures[point.creature as usize].key.clone())
                .or_insert(0) += 1;
        }
    }
    assert_eq!(realized, expected);
    assert_eq!(table.point_count(), 398);

    let world = input.world.clone();
    for bytes in shards(&root, "content/npcs/definitions", "npcs-")? {
        families.add_npcs(&bytes)?;
    }
    for bytes in shards(&root, "content/dialogues/definitions", "dialogues-")? {
        families.add_dialogues(&bytes)?;
    }
    for bytes in shards(&root, "content/world/npc-placements", "placements-")? {
        families.add_npc_placements(&bytes)?;
    }
    families
        .npc_placements
        .retain(|placement| placement.npc == "oteryn:npc.mayor_pocaro");
    assert_eq!(families.npc_placements.len(), 1);
    assert_eq!(families.npc_placements[0].cell, (34073, 32504, 6));
    let mayor_regions = vec![fs::read(
        root.join("content/world/placements/region-z06-x133-y126.b3"),
    )?];
    let input = Input {
        regions: &mayor_regions,
        palette: &palette,
        identity: Identity::default(),
        world,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &families,
    };
    let (npcs, report) = realize_npcs(&input, &registry, &table)?;
    assert!(report.held.is_empty(), "{report:#?}");
    assert_eq!(npcs.placement_count(), 1);
    assert_eq!(npcs.npcs[0].key, "oteryn:npc.mayor_pocaro");
    let at = npcs.npcs[0].placements[0];
    assert_eq!((at.x, at.y, at.floor), (34073, 32504, -6));
    Ok(())
}
