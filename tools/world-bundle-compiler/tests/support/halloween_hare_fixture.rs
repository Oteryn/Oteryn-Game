// Shared actual source-Hare compiler fixture; no synthetic tile or placement.
use std::fs;
use std::path::Path;

use oteryn_world_bundle_compiler::bundle::{BuildClass, Identity};
use oteryn_world_bundle_compiler::compile::Input;
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

pub fn compile(root: &Path) -> TestResult<Vec<u8>> {
    let mut families = Families::default();
    for bytes in shards(root, "content/houses", "houses-")? {
        families.add_houses(&bytes)?;
    }
    for bytes in shards(root, "content/world/transitions", "teleports-")? {
        families.add_teleports(&bytes)?;
    }
    families
        .teleports
        .retain(|&(x, y, z), _| x / 256 == 127 && y / 256 == 125 && z == 7);
    let regions = (7..=7)
        .map(|z| {
            fs::read(root.join(format!(
                "content/world/placements/region-z{z:02}-x127-y125.b3"
            )))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let index: Value =
        serde_json::from_slice(&fs::read(root.join("content/world/placements/index.json"))?)?;
    let mut registry = Registry::default();
    for bytes in shards(root, "content/items/definitions", "items-")? {
        registry.add_items(&bytes)?;
    }
    registry.add_item_profiles(&fs::read(
        root.join("content/items/definitions/spell-native-profiles.json"),
    )?)?;
    for (family, directory, prefix) in [
        ("Terrain", "content/world/terrain", "terrain-"),
        ("WorldObject", "content/world/objects", "objects-"),
    ] {
        for bytes in shards(root, directory, prefix)? {
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
    let worlds = shards(root, "content/world/worlds", "worlds-")?;
    let input = Input {
        regions: &regions,
        palette: &palette,
        identity: Identity::default(),
        world: project::world_extent(worlds.first().ok_or("World shard")?)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &families,
    };
    let built = oteryn_world_bundle_compiler::compile::compile(&input, &registry)?;
    let bundle = oteryn_world_bundle_compiler::bundle::read(&built.bytes)?;
    assert!(
        !bundle.manifest.palette.is_empty(),
        "actual source region palette"
    );
    Ok(built.bytes)
}
