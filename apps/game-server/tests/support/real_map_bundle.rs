//! MAP-VIEWPORT-MEASURE-1: the real map of the checked-in World Project, compiled for a
//! measurement. The compile follows `map_load_budget_compile_the_real_map` of `map_load_base.rs`
//! (MAP-LOAD-1): every placement region of `content/world/placements`, the checked families, and
//! a resolver for which every palette key resolves. It differs in one thing: the compact id of a
//! palette key is its palette index + 1, so a facts table is a dense array, as the server's
//! palette table would be.
//!
//! Mounted with `#[path]` by `gameplay_transport/world_map_real_tests.rs`; `map_load_base.rs`
//! keeps its own copy until a later slice moves it (it is not an owned path of this task).

use oteryn_world_bundle_compiler::Error;
use oteryn_world_bundle_compiler::bundle::{BuildClass, Extent, Family, Identity, Terrain};
use oteryn_world_bundle_compiler::compile::{Input, KeyResolver, Resolution, compile};
use oteryn_world_bundle_compiler::project::Families;
use std::collections::{HashMap, HashSet};
use std::error::Error as StdError;
use std::path::{Path, PathBuf};

pub type Fallible<T> = Result<T, Box<dyn StdError>>;

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The real map compiled: bundle bytes, digest, and the palette (compact id = index + 1).
pub struct RealMap {
    pub bytes: Vec<u8>,
    pub digest: [u8; 32],
    pub keys: Vec<String>,
    /// The Terrain record of each palette key that has a known kind.
    pub terrain: HashMap<String, Terrain>,
}

struct Measurement {
    ids: HashMap<String, u32>,
    terrain: HashMap<String, Terrain>,
}

impl KeyResolver for Measurement {
    fn resolve(&self, key: &str) -> Resolution {
        let family = if self.terrain.contains_key(key) {
            Family::Terrain
        } else {
            Family::Item
        };
        match self.ids.get(key) {
            Some(id) => Resolution::Resolved(family, *id),
            None => Resolution::Unknown,
        }
    }

    fn terrain(&self, key: &str) -> Result<Option<Terrain>, Error> {
        Ok(self.terrain.get(key).copied())
    }

    fn floor_change(&self, _: &str) -> bool {
        false
    }
}

fn read_json(path: &Path) -> Fallible<serde_json::Value> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

/// The Terrain records of `content/world/terrain` with a known kind, by item key; a ground takes
/// its known walkability and ground speed, else walkable at 150.
fn terrain_records(root: &Path) -> Fallible<HashMap<String, Terrain>> {
    use oteryn_world_bundle_compiler::bundle::TerrainKind;
    let mut terrain = HashMap::new();
    for entry in std::fs::read_dir(root.join("content/world/terrain"))? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if !name.starts_with("terrain-") {
            continue;
        }
        let shard = read_json(&path)?;
        for record in shard["records"].as_array().ok_or("records")? {
            let known = |field: &str| {
                (record[field]["state"] == "KNOWN").then(|| record[field]["value"].clone())
            };
            let kind = match known("kind").as_ref().and_then(|v| v.as_str()) {
                Some("ground") => TerrainKind::Ground,
                Some("border") => TerrainKind::Border,
                Some("wall") => TerrainKind::Wall,
                Some("roof") => TerrainKind::Roof,
                Some("field") => TerrainKind::Field,
                Some("common") => TerrainKind::Common,
                _ => continue,
            };
            let item = record["provenance"]["item_pointer"]["key"]
                .as_str()
                .ok_or("item pointer")?;
            let (walkable, ground_speed) = if kind == TerrainKind::Ground {
                let walkable = known("walkable").and_then(|v| v.as_bool()).unwrap_or(true);
                let speed = known("ground_speed")
                    .and_then(|v| v.as_u64())
                    .and_then(|v| u16::try_from(v).ok())
                    .filter(|v| *v <= 1000 && (*v >= 1 || !walkable))
                    .unwrap_or(150);
                (Some(walkable), Some(speed))
            } else {
                (None, None)
            };
            terrain.insert(
                item.to_owned(),
                Terrain {
                    kind,
                    walkable,
                    ground_speed,
                },
            );
        }
    }
    Ok(terrain)
}

/// The shards of one family directory whose file name starts with `prefix`, sorted by path.
pub fn shards(root: &Path, directory: &str, prefix: &str) -> Fallible<Vec<Vec<u8>>> {
    let mut paths: Vec<_> = std::fs::read_dir(root.join(directory))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()?;
    paths.retain(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".json"))
    });
    paths.sort();
    Ok(paths.iter().map(std::fs::read).collect::<Result<_, _>>()?)
}

/// Compiles every placement region of `content/world/placements` with the checked families.
pub fn compile_real_map() -> Fallible<RealMap> {
    let root = repo_root();
    let index = read_json(&root.join("content/world/placements/index.json"))?;
    let regions = index["shards"]
        .as_array()
        .ok_or("shards")?
        .iter()
        .map(|path| {
            Ok(std::fs::read(
                root.join(path.as_str().ok_or("shard path")?),
            )?)
        })
        .collect::<Result<Vec<_>, Box<dyn StdError>>>()?;
    let keys: Vec<String> = index["palette"]
        .as_array()
        .ok_or("palette")?
        .iter()
        .map(|entry| entry["key"].as_str().map(String::from).ok_or("key"))
        .collect::<Result<_, _>>()?;
    let world: Extent = oteryn_world_bundle_compiler::project::world_extent(&std::fs::read(
        root.join("content/world/worlds/worlds-00000-00000.json"),
    )?)?;
    let terrain = terrain_records(&root)?;
    let ids = keys
        .iter()
        .enumerate()
        .map(|(index, key)| (key.clone(), index as u32 + 1))
        .collect();
    let resolver = Measurement { ids, terrain };
    let mut families = Families::default();
    for shard in shards(&root, "content/world/transitions", "teleports-")? {
        families.add_teleports(&shard)?;
    }
    for shard in shards(&root, "content/houses", "houses-")? {
        families.add_houses(&shard)?;
    }
    for shard in shards(&root, "content/creatures/definitions", "creatures-")? {
        families.add_creatures(&shard)?;
    }
    for shard in shards(&root, "content/world/spawns", "spawns-")? {
        families.add_spawns(&shard)?;
    }
    let input = Input {
        regions: &regions,
        palette: &keys,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            ..Identity::default()
        },
        world,
        build_class: BuildClass::Production,
        draft_areas: Vec::new(),
        families: &families,
    };
    let compiled = compile(&input, &resolver)?;
    Ok(RealMap {
        bytes: compiled.bytes,
        digest: compiled.digest,
        keys,
        terrain: resolver.terrain,
    })
}

/// The item keys whose definition says `pickupable` is KNOWN true.
pub fn pickupable_keys() -> Fallible<HashSet<String>> {
    let root = repo_root();
    let mut keys = HashSet::new();
    for shard in shards(&root, "content/items/definitions", "items-")? {
        let shard: serde_json::Value = serde_json::from_slice(&shard)?;
        for record in shard["records"].as_array().ok_or("records")? {
            let physical = &record["definition"]["semantics"]["physical"]["value"]["pickupable"];
            if physical["state"] == "KNOWN"
                && physical["value"] == true
                && let Some(key) = record["definition"]["identity"]["key"].as_str()
            {
                keys.insert(key.to_owned());
            }
        }
    }
    Ok(keys)
}
