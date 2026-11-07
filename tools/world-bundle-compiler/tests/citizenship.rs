//! W23: real civic portal and temple cells survive native terrain admission.
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use oteryn_world_bundle_compiler::bundle::{BuildClass, Identity};
use oteryn_world_bundle_compiler::compile::{Input, realize_npcs};
use oteryn_world_bundle_compiler::project::{self, Families, TravelRoute};
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
fn w23_shipped_citizen_portals_temples_and_roof_landings_have_native_ground() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source: Value = serde_json::from_slice(&fs::read(
        root.join("tools/content-schema/world-authoring/samples/citizen-bindings.json"),
    )?)?;
    let mut positions = BTreeSet::new();
    for row in source["bindings"].as_array().ok_or("bindings")? {
        let p = row["position"].as_array().ok_or("position")?;
        positions.insert((
            u16::try_from(p[0].as_u64().ok_or("x")?)?,
            u16::try_from(p[1].as_u64().ok_or("y")?)?,
            u8::try_from(p[2].as_u64().ok_or("z")?)?,
        ));
    }
    assert_eq!(positions.len(), 18);
    // The exact template lookup is independently exercised by the durable dispatch case.
    // Here the shipped hometown catalogue provides the real temple destinations.
    for bytes in shards(&root, "content/world/areas/cities", "areas-")? {
        let shard: Value = serde_json::from_slice(&bytes)?;
        for row in shard["areas"].as_array().ok_or("areas")? {
            let temple = &row["hometown"]["temple"];
            if !temple.is_null()
                && row["provenance"]["hometown"]["engine_town"]["town_id"]
                    .as_u64()
                    .is_some_and(|id| id >= 5)
            {
                positions.insert((
                    u16::try_from(temple["x"].as_u64().ok_or("temple x")?)?,
                    u16::try_from(temple["y"].as_u64().ok_or("temple y")?)?,
                    u8::try_from(temple["z"].as_u64().ok_or("temple z")?)?,
                ));
            }
        }
    }
    positions.extend([(32210, 31134, 7), (32212, 31131, 5)]);
    let mut regions = BTreeSet::new();
    let mut families = Families::default();
    for (i, &(x, y, z)) in positions.iter().enumerate() {
        regions.insert(format!(
            "content/world/placements/region-z{z:02}-x{:03}-y{:03}.b3",
            x / 256,
            y / 256
        ));
        families.routes.push(TravelRoute {
            service: "oteryn:service.citizen-test".into(),
            route: format!("cell-{i}"),
            destination: (x, y, z),
        });
    }
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
        .map(|row| {
            let key = row["key"].as_str().ok_or("palette key")?.to_owned();
            if row["provisional"] == true {
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
    let (_, report) = realize_npcs(&input, &registry, &Default::default())?;
    assert!(report.routes_held.is_empty(), "{report:#?}");
    Ok(())
}
