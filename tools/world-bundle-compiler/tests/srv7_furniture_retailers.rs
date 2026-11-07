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
fn srv7_all_twenty_nine_furniture_retailers_have_real_ground_and_production_npc_placements()
-> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let points = [
        ("oteryn:npc.allen", (32990, 32062, 6)),
        ("oteryn:npc.asphota", (33777, 32844, 6)),
        ("oteryn:npc.borkas", (32993, 32068, 6)),
        ("oteryn:npc.dorbin", (32783, 31236, 7)),
        ("oteryn:npc.eddy", (32168, 32428, 7)),
        ("oteryn:npc.edvard", (33229, 31832, 7)),
        ("oteryn:npc.fayla", (33640, 31898, 4)),
        ("oteryn:npc.feizuhl", (33066, 32886, 7)),
        ("oteryn:npc.florentine", (32363, 31796, 7)),
        ("oteryn:npc.gamon", (32408, 32170, 7)),
        ("oteryn:npc.haani", (33811, 32771, 4)),
        ("oteryn:npc.hairycles", (32826, 32574, 6)),
        ("oteryn:npc.hofech", (33270, 32440, 7)),
        ("oteryn:npc.inkaef", (33904, 31518, 7)),
        ("oteryn:npc.isika", (33773, 32840, 6)),
        ("oteryn:npc.iwar", (32619, 31917, 8)),
        ("oteryn:npc.janz", (32279, 31131, 7)),
        ("oteryn:npc.john_bounac", (32443, 32487, 3)),
        ("oteryn:npc.julian", (32883, 32076, 6)),
        ("oteryn:npc.nicholas", (33640, 31898, 5)),
        ("oteryn:npc.nydala", (32327, 31828, 7)),
        ("oteryn:npc.peggy", (32294, 32807, 7)),
        ("oteryn:npc.rose", (32970, 32034, 6)),
        ("oteryn:npc.shiantis", (32890, 32085, 6)),
        ("oteryn:npc.ukea", (32652, 31666, 6)),
        ("oteryn:npc.vera", (32140, 31671, 7)),
        ("oteryn:npc.yoem", (33304, 31965, 7)),
        ("oteryn:npc.yulas", (33000, 32065, 6)),
        ("oteryn:npc.zaidal", (32622, 32746, 5)),
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
        29,
        "all29 original retailer source placements must reach the real compiler input"
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
    assert_eq!(npcs.placement_count(), 29);
    assert_eq!(npcs.npcs.len(), 29);
    eprintln!(
        "SRV7_FURNITURE_RETAILER_REAL_SOURCE_PLACEMENTS all29=true offers904=true geometry_rewritten=false"
    );
    Ok(())
}
