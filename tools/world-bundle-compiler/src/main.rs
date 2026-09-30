//! `oteryn-world-bundle-compiler parity <repository root>`: the MAP-BUNDLE-1 parity report of
//! the World Project against its Transition.Teleport and House families, as JSON on stdout.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use oteryn_world_bundle_compiler::Error;
use oteryn_world_bundle_compiler::compile::parity;
use oteryn_world_bundle_compiler::project::{self, Families};

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

fn run(root: &Path) -> Result<String, Error> {
    let read =
        |path: &Path| fs::read(path).map_err(|e| Error::Format(format!("{}: {e}", path.display())));
    let index: serde_json::Value =
        serde_json::from_slice(&read(&root.join("content/world/placements/index.json"))?)
            .map_err(|e| Error::Format(format!("placements index: {e}")))?;
    let regions = index
        .get("shards")
        .and_then(|shards| shards.as_array())
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
    let mut report = serde_json::to_value(parity(&regions, &families)?)
        .map_err(|e| Error::Format(e.to_string()))?;
    report["draft_areas"] = project::draft_areas(&index)?.into();
    serde_json::to_string_pretty(&report).map_err(|e| Error::Format(e.to_string()))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let [_, command, root] = args.as_slice() else {
        eprintln!("usage: oteryn-world-bundle-compiler parity <repository root>");
        return ExitCode::from(2);
    };
    if command != "parity" {
        eprintln!("unknown command `{command}`");
        return ExitCode::from(2);
    }
    match run(Path::new(root)) {
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
