//! W18: actual production placement input, catalogue admission and original source ground.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{Input, realize_npcs, realize_spawns},
    project::{self, Families},
    resolve::Registry,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::{fs, path::Path};
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
fn srv7_gnomilly_and_norbert_support_npcs_have_original_source_placements() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let points = [
        ("oteryn:npc.gnomilly", (32242, 32599, 9)),
        ("oteryn:npc.norbert", (32952, 32103, 6)),
    ];
    let keys = points.iter().map(|(key, _)| *key).collect::<BTreeSet<_>>();
    let mut families = Families::default();
    for bytes in shards(&root, "content/npcs/definitions", "npcs-")? {
        families.add_npcs(&bytes)?;
    }
    for bytes in shards(&root, "content/dialogues/definitions", "dialogues-")? {
        families.add_dialogues(&bytes)?;
    }
    for bytes in shards(&root, "content/services/trade", "trade-")? {
        families.add_services(&bytes)?;
    }
    for bytes in shards(&root, "content/world/npc-placements", "placements-")? {
        families.add_npc_placements(&bytes)?;
    }
    families
        .npc_placements
        .retain(|p| keys.contains(p.npc.as_str()));
    assert_eq!(
        families.npc_placements.len(),
        2,
        "Gnomilly Spike support source placements must reach the real compiler input"
    );
    for (key, cell) in &points {
        assert!(
            families
                .npc_placements
                .iter()
                .any(|p| p.npc == *key && p.cell == *cell),
            "wrong/missing actual retailer source cell for{key}"
        );
    }
    let regions = points
        .iter()
        .map(|(_, (x, y, z))| {
            format!(
                "content/world/placements/region-z{z:02}-x{:03}-y{:03}.b3",
                x / 256,
                y / 256
            )
        })
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
    for (family, dir, prefix) in [
        ("Terrain", "content/world/terrain", "terrain-"),
        ("WorldObject", "content/world/objects", "objects-"),
    ] {
        for bytes in shards(&root, dir, prefix)? {
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
        draft_areas: vec![],
        families: &families,
    };
    let (spawns, _) = realize_spawns(&input, &registry)?;
    let (npcs, report) = realize_npcs(&input, &registry, &spawns)?;
    assert!(
        report.held.is_empty(),
        "real source NPC/ground admission cannot be bypassed: {report:#?}"
    );
    assert_eq!(npcs.placement_count(), 2);
    assert_eq!(npcs.npcs.len(), 2);
    eprintln!("SRV7_GNOMILLY_SOURCE_PLACEMENT geometry_rewritten=false");
    Ok(())
}
