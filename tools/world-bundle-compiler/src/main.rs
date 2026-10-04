//! `oteryn-world-bundle-compiler parity <repository root>`: the MAP-BUNDLE-1 parity report of
//! the World Project against its Transition.Teleport and House families, as JSON on stdout.
//!
//! `oteryn-world-bundle-compiler compile <repository root> <identity.json> <out> <build class>`:
//! compiles the World Project into a server World Bundle at `<out>`, proves it tile by tile
//! against its source, and prints a JSON summary on stdout.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use oteryn_world_bundle_compiler::Error;
use oteryn_world_bundle_compiler::bundle::{BuildClass, Identity};
use oteryn_world_bundle_compiler::compile::{self, Input, equivalence, parity, placed_palette};
use oteryn_world_bundle_compiler::project::{self, Families};
use oteryn_world_bundle_compiler::resolve::Registry;
use serde_json::{Value, json};

/// The shards of `directory` whose file name starts with `prefix`, in name order.
fn shards(directory: &Path, prefix: &str) -> Result<Vec<Vec<u8>>, Error> {
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

fn read(path: &Path) -> Result<Vec<u8>, Error> {
    fs::read(path).map_err(|e| Error::Format(format!("{}: {e}", path.display())))
}

fn json(bytes: &[u8], what: &str) -> Result<Value, Error> {
    serde_json::from_slice(bytes).map_err(|e| Error::Format(format!("{what}: {e}")))
}

/// The World Project of `root`: placements index, B3 regions and the checked families.
struct Project {
    index: Value,
    regions: Vec<Vec<u8>>,
    families: Families,
}

fn project(root: &Path) -> Result<Project, Error> {
    let index = json(
        &read(&root.join("content/world/placements/index.json"))?,
        "placements index",
    )?;
    let regions = index
        .get("shards")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Format("placements index has no shards".into()))?
        .iter()
        .map(|path| match path.as_str() {
            Some(path) => read(&root.join(path)),
            None => Err(Error::Format("placements shard is not a path".into())),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut families = Families::default();
    for shard in shards(&root.join("content/world/transitions"), "teleports-")? {
        families.add_teleports(&shard)?;
    }
    for shard in shards(&root.join("content/houses"), "houses-")? {
        families.add_houses(&shard)?;
    }
    Ok(Project {
        index,
        regions,
        families,
    })
}

/// The Item registry, Terrain and WorldObject catalogues and the palette keys of `project`.
fn registry(root: &Path, project: &Project) -> Result<(Registry, Vec<String>), Error> {
    let mut registry = Registry::default();
    for shard in shards(&root.join("content/items/definitions"), "items-")? {
        registry.add_items(&shard)?;
    }
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

fn run_parity(root: &Path) -> Result<Value, Error> {
    let project = project(root)?;
    let mut report = serde_json::to_value(parity(&project.regions, &project.families)?)
        .map_err(|e| Error::Format(e.to_string()))?;
    report["draft_areas"] = project::draft_areas(&project.index)?.into();
    // Terrain classes of the placed palette entries (format v2, decision §1.4).
    let (registry, palette) = registry(root, &project)?;
    let placed = placed_palette(&project.regions)?;
    let keys = placed
        .iter()
        .filter_map(|at| palette.get(*at as usize).map(String::as_str));
    report["terrain"] = serde_json::to_value(registry.terrain_counts(keys))
        .map_err(|e| Error::Format(e.to_string()))?;
    Ok(report)
}

fn run_compile(root: &Path, identity: &Path, out: &Path, class: &str) -> Result<Value, Error> {
    let build_class = match class {
        "production" => BuildClass::Production,
        "non-production" => BuildClass::NonProduction,
        other => return Err(Error::Format(format!("unknown build class `{other}`"))),
    };
    let identity: Identity = serde_json::from_slice(&read(identity)?)
        .map_err(|e| Error::Format(format!("identity: {e}")))?;
    let project = project(root)?;
    let (registry, palette) = registry(root, &project)?;
    let worlds = shards(&root.join("content/world/worlds"), "worlds-")?;
    let [world] = worlds.as_slice() else {
        return Err(Error::Format(
            "the World Project must hold one World shard".into(),
        ));
    };
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity,
        world: project::world_extent(world)?,
        build_class,
        draft_areas: project::draft_areas(&project.index)?,
        families: &project.families,
    };
    let compiled = compile::compile(&input, &registry)?;
    let proof = equivalence(&input, &registry, &compiled.bytes)?;
    fs::write(out, &compiled.bytes)
        .map_err(|e| Error::Format(format!("{}: {e}", out.display())))?;
    let digest: String = compiled.digest.iter().map(|b| format!("{b:02x}")).collect();
    Ok(json!({
        "digest": digest,
        "bytes": compiled.bytes.len(),
        "skipped_provisional_entries": compiled.diagnostics.len(),
        "dropped_teleports": compiled.dropped_teleports.len(),
        "equivalence": proof,
    }))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let result = match args.as_slice() {
        [_, command, root] if command == "parity" => run_parity(Path::new(root)),
        [_, command, root, identity, out, class] if command == "compile" => {
            run_compile(Path::new(root), Path::new(identity), Path::new(out), class)
        }
        _ => {
            eprintln!(
                "usage: oteryn-world-bundle-compiler parity <repository root>\n       \
                 oteryn-world-bundle-compiler compile <repository root> <identity.json> <out> \
                 <production|non-production>"
            );
            return ExitCode::from(2);
        }
    };
    match result.and_then(|report| {
        serde_json::to_string_pretty(&report).map_err(|e| Error::Format(e.to_string()))
    }) {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
