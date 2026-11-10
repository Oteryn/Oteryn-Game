//! Read the canonical World Project inputs used by the bundle compiler.
use crate::bundle::Extent;
use crate::{
    Error, compile,
    project::{self, Families},
    resolve::Registry,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
/// The shards of `directory` whose file name starts with `prefix`, in name order.
pub fn shards(directory: &Path, prefix: &str) -> Result<Vec<Vec<u8>>, Error> {
    let io = |e: std::io::Error| Error::Format(format!("{}: {e}", directory.display()));
    let mut paths: Vec<PathBuf> = fs::read_dir(directory)
        .map_err(io)?
        .map(|entry| entry.map(|entry| entry.path()).map_err(io))
        .collect::<Result<_, _>>()?;
    paths.retain(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".json"))
    });
    paths.sort();
    paths
        .iter()
        .map(|path| fs::read(path).map_err(io))
        .collect()
}

pub fn read(path: &Path) -> Result<Vec<u8>, Error> {
    fs::read(path).map_err(|e| Error::Format(format!("{}: {e}", path.display())))
}

pub fn json(bytes: &[u8], what: &str) -> Result<Value, Error> {
    serde_json::from_slice(bytes).map_err(|e| Error::Format(format!("{what}: {e}")))
}

/// A family index and its shards in index order.
type Indexed = (Value, Vec<Vec<u8>>);

/// The `OTERYN_FAMILY_INDEX/v1` index of `directory` (relative to `root`) and its shards in
/// index order; `None` without an index or for another schema (a tree directory marker).
pub fn indexed(root: &Path, directory: &str) -> Result<Option<Indexed>, Error> {
    let path = root.join(directory).join("index.json");
    if !path.is_file() {
        return Ok(None);
    }
    let index = json(&read(&path)?, directory)?;
    if index["schema"] != "OTERYN_FAMILY_INDEX/v1" {
        return Ok(None);
    }
    let shards = index["shards"]
        .as_array()
        .ok_or_else(|| Error::Format(format!("{directory} index has no shards")))?
        .iter()
        .map(|shard| match shard.as_str() {
            Some(shard) if shard.starts_with(directory) => read(&root.join(shard)),
            _ => Err(Error::Format(format!(
                "{directory} shard is not one of its paths"
            ))),
        })
        .collect::<Result<_, _>>()?;
    Ok(Some((index, shards)))
}

/// The NPC, Dialogue and Service records, the travel routes, the `Npc.Placement` family with
/// its held list (NPC-PLACE-1 §4) and the catalogue digest (§3.1).
fn add_npc_families(root: &Path, families: &mut Families) -> Result<(), Error> {
    let shards_of = |directory: &str| {
        indexed(root, directory).map(|found| found.map(|(_, shards)| shards).unwrap_or_default())
    };
    for shard in shards_of("content/npcs/definitions/")? {
        families.add_npcs(&shard)?;
    }
    for shard in shards_of("content/dialogues/definitions/")? {
        families.add_dialogues(&shard)?;
    }
    let services = root.join("content/services");
    let io = |e: std::io::Error| Error::Format(format!("content/services: {e}"));
    let mut kinds: Vec<String> = fs::read_dir(&services)
        .map_err(io)?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_, _>>()
        .map_err(io)?;
    kinds.sort();
    for kind in kinds {
        for shard in shards_of(&format!("content/services/{kind}/"))? {
            families.add_services(&shard)?;
        }
    }
    if let Some((index, shards)) = indexed(root, "content/world/npc-placements/")? {
        for shard in shards {
            families.add_npc_placements(&shard)?;
        }
        let held = &index["held"];
        let path = held["path"]
            .as_str()
            .filter(|path| path.starts_with("content/world/npc-placements/"))
            .ok_or_else(|| Error::Format("NPC placements index has no held path".into()))?;
        let bytes = read(&root.join(path))?;
        if held["sha256"] != hex(&Sha256::digest(&bytes)) {
            return Err(Error::Format(
                "NPC held list sha256 differs from its index".into(),
            ));
        }
        let before = families.npc_source_held.len();
        families.add_npc_source_held(&bytes)?;
        if held["count"] != (families.npc_source_held.len() - before) {
            return Err(Error::Format(
                "NPC held list count differs from its index".into(),
            ));
        }
    }
    families.catalogue_sha256 = Some(
        crate::npc::catalogue_sha256(&root.join("content"))
            .map_err(|e| Error::Format(format!("NPC catalogue: {e}")))?,
    );
    Ok(())
}

/// The World Project of `root`: placements index, B3 regions and the checked families.
pub struct Project {
    pub index: Value,
    pub regions: Vec<Vec<u8>>,
    pub families: Families,
}

pub fn project(root: &Path) -> Result<Project, Error> {
    project_selected(root, None)
}

/// Compile a non-production selection of real B3 regions, keeping every native floor.
pub fn project_regions(root: &Path, selected: &[(u16, u16)]) -> Result<Project, Error> {
    if selected.is_empty() || selected.iter().any(|&(x, y)| x > 255 || y > 255) {
        return Err(Error::Format("invalid World region selection".into()));
    }
    project_selected(root, Some(selected))
}

fn project_selected(root: &Path, selected: Option<&[(u16, u16)]>) -> Result<Project, Error> {
    let mut index = json(
        &read(&root.join("content/world/placements/index.json"))?,
        "placements index",
    )?;
    let mut regions = index
        .get("shards")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Format("placements index has no shards".into()))?
        .iter()
        .filter(|path| {
            selected.is_none_or(|selected| {
                path.as_str().is_none_or(|path| {
                    selected
                        .iter()
                        .any(|&(x, y)| path.ends_with(&format!("-x{x:03}-y{y:03}.b3")))
                })
            })
        })
        .map(|path| match path.as_str() {
            Some(path) => read(&root.join(path)),
            None => Err(Error::Format("placements shard is not a path".into())),
        })
        .collect::<Result<Vec<_>, _>>()?;
    crate::authored_props::apply(root, &mut index, &mut regions, selected)?;
    let mut families = Families::default();
    for shard in shards(&root.join("content/world/transitions"), "teleports-")? {
        families.add_teleports(&shard)?;
    }
    if let Some(selected) = selected {
        families
            .teleports
            .retain(|&(x, y, _), _| selected.contains(&(x / 256, y / 256)));
    }
    for shard in shards(&root.join("content/houses"), "houses-")? {
        families.add_houses(&shard)?;
    }
    for shard in shards(&root.join("content/creatures/definitions"), "creatures-")? {
        families.add_creatures(&shard)?;
    }
    for shard in shards(&root.join("content/world/spawns"), "spawns-")? {
        families.add_spawns(&shard)?;
    }
    add_npc_families(root, &mut families)?;
    families.subareas = compile::subareas::Source::load(root)?;
    families.layers = compile::layers::Source::load(root)?;
    Ok(Project {
        index,
        regions,
        families,
    })
}

/// The Item registry, Terrain and WorldObject catalogues and the palette keys of `project`.
pub fn registry(root: &Path, project: &Project) -> Result<(Registry, Vec<String>), Error> {
    let mut registry = Registry::default();
    for shard in shards(&root.join("content/items/definitions"), "items-")? {
        registry.add_items(&shard)?;
    }
    registry.add_item_profiles(&read(
        &root.join("content/items/definitions/spell-native-profiles.json"),
    )?)?;
    for (family, directory, prefix) in [
        ("Terrain", "content/world/terrain", "terrain-"),
        ("WorldObject", "content/world/objects", "objects-"),
    ] {
        for shard in shards(&root.join(directory), prefix)? {
            registry.add_catalogue(family, &shard)?;
        }
    }
    let mut palette = Vec::new();
    for entry in project.index["palette"]
        .as_array()
        .ok_or_else(|| Error::Format("placements index has no palette".into()))?
    {
        let key = entry["key"]
            .as_str()
            .ok_or_else(|| Error::Format("palette entry without a key".into()))?;
        if entry["provisional"] == Value::Bool(true) {
            registry.add_provisional(key.to_owned());
        }
        palette.push(key.to_owned());
    }
    registry.seal()?;
    Ok((registry, palette))
}

/// The World shard of `root`.
pub fn world_extent(root: &Path) -> Result<Extent, Error> {
    let worlds = shards(&root.join("content/world/worlds"), "worlds-")?;
    let [world] = worlds.as_slice() else {
        return Err(Error::Format(
            "the World Project must hold one World shard".into(),
        ));
    };
    project::world_extent(world)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_selection_refuses_empty_and_out_of_world_coordinates() {
        for selected in [&[][..], &[(256, 125)][..], &[(126, 256)][..]] {
            assert!(
                matches!(project_regions(Path::new("/absent-world-source"), selected), Err(Error::Format(message)) if message == "invalid World region selection")
            );
        }
    }
}
