//! `oteryn-world-bundle-compiler parity <repository root>`: the MAP-BUNDLE-1 parity report of
//! the World Project against its Transition.Teleport and House families, as JSON on stdout.
//!
//! `oteryn-world-bundle-compiler compile <repository root> <identity.json> <out> <build class>`:
//! compiles the World Project into a server World Bundle at `<out>`, proves it tile by tile
//! against its source, and prints a JSON summary on stdout.
//!
//! `oteryn-world-bundle-compiler derive-identity <repository root> <world slug>`: prints the
//! compiler `Identity` of the World from its named sources (decision ARCH-WORLD-CONTENT-SERVE-1
//! §1.2), refusing a `required_features` value the node does not support.
//!
//! `oteryn-world-bundle-compiler pin-check <repository root> <out directory>`: checks every
//! World pin under `content/world/pins/` against the tree, builds each bundle twice, and writes
//! it to `<out directory>` with `artifact-name.txt`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use oteryn_world_bundle_compiler::Error;
use oteryn_world_bundle_compiler::bundle::{
    self, BuildClass, Extent, Identity, Manifest, RUNTIME_VERSION, SUPPORTED_CAPABILITIES,
    TerrainKind,
};
use oteryn_world_bundle_compiler::compile::{
    self, Input, SpawnReport, equivalence, parity, placed_palette,
};
use oteryn_world_bundle_compiler::project::{self, Families};
use oteryn_world_bundle_compiler::resolve::Registry;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

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
    for shard in shards(&root.join("content/creatures/definitions"), "creatures-")? {
        families.add_creatures(&shard)?;
    }
    for shard in shards(&root.join("content/world/spawns"), "spawns-")? {
        families.add_spawns(&shard)?;
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

/// The World shard of `root`.
fn world_extent(root: &Path) -> Result<Extent, Error> {
    let worlds = shards(&root.join("content/world/worlds"), "worlds-")?;
    let [world] = worlds.as_slice() else {
        return Err(Error::Format(
            "the World Project must hold one World shard".into(),
        ));
    };
    project::world_extent(world)
}

/// The spawn report as JSON: totals, the count per reason and every point left out.
fn spawn_report(report: &SpawnReport) -> Result<Value, Error> {
    let mut value = serde_json::to_value(report).map_err(|e| Error::Format(e.to_string()))?;
    let mut reasons = std::collections::BTreeMap::<String, usize>::new();
    for dropped in &report.dropped {
        *reasons.entry(format!("{:?}", dropped.reason)).or_default() += 1;
    }
    value["dropped_by_reason"] = json!(reasons);
    Ok(value)
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
    // The spawn family against the cells it stands on (CREATURE-AI-0 §6.1). A terrain record
    // that is not classified yet is reported per point, not a failure here.
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: world_extent(root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    report["spawns"] = spawn_report(&compile::realize_spawns(&input, &registry)?.1)?;
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
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity,
        world: world_extent(root)?,
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
        "spawns": spawn_report(&compiled.spawns)?,
        "equivalence": proof,
    }))
}

/// The repository paths the compiler reads or builds from: a path is a directory prefix when it
/// ends with `/`. `tools/repository/classify_pr_test_lanes.py` lists the same paths
/// (`WORLD_BUNDLE_INPUT_PATHS`); a test keeps the two equal. A PR that adds a build input adds
/// its path to both lists.
const INPUT_PATHS: &[&str] = &[
    "content/world/",
    "content/houses/",
    "content/creatures/definitions/",
    "content/items/definitions/",
    "tools/world-bundle-compiler/",
    "crates/world-bundle/",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo/",
];
const INPUTS_DOMAIN: &str = "oteryn:world-bundle/inputs/v1";
const PINS: &str = "content/world/pins";
const PIN_SCHEMA: &str = "OTERYN_WORLD_PIN/v1";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn text(value: &Value, key: &str, what: &str) -> Result<String, Error> {
    value[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| Error::Format(format!("{what}: `{key}` is missing or not a string")))
}

/// A pin file is `<slug>.json`; the identity and ruleset files of a World are inputs.
fn is_pin_file(name: &str) -> bool {
    name.ends_with(".json") && !name.ends_with(".identity.json") && !name.ends_with(".ruleset.json")
}

/// The reviewed pin of one World (`content/world/pins/<world slug>.json`).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pin {
    schema: String,
    digest: String,
    project_format_version: String,
    world_schema_version: String,
    content_revision: String,
    production: bool,
    entry_start: EntryStart,
    inputs_digest: String,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct EntryStart {
    x: u16,
    y: u16,
    floor: i8,
}

/// The compiler `Identity` of `slug`, each field from its one named source.
fn derive_identity(root: &Path, slug: &str) -> Result<Identity, Error> {
    let world = root.join("content/world");
    let project = json(&read(&world.join("project.json"))?, "project.json")?;
    let manifest = json(&read(&world.join("manifest.json"))?, "manifest.json")?;
    let lock_bytes = read(&world.join("content.lock.json"))?;
    let lock = json(&lock_bytes, "content.lock.json")?;
    let ruleset = json(
        &read(&root.join(PINS).join(format!("{slug}.ruleset.json")))?,
        "ruleset file",
    )?;

    let lock_digest = hex(&Sha256::digest(&lock_bytes));
    if text(&project, "content_lock_sha256", "project.json")? != lock_digest {
        return Err(Error::Format(
            "project.json content_lock_sha256 is not the SHA-256 of content.lock.json".into(),
        ));
    }
    let strings = |value: &Value, key: &str, what: &str| -> Result<Vec<String>, Error> {
        value[key]
            .as_array()
            .ok_or_else(|| Error::Format(format!("{what}: `{key}` is not a list")))?
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| Error::Format(format!("{what}: `{key}` holds a non-string")))
            })
            .collect()
    };
    let sorted_unique = |list: &[String]| list.windows(2).all(|pair| pair[0] < pair[1]);
    let required_capabilities = strings(&manifest, "required_features", "manifest.json")?;
    if !sorted_unique(&required_capabilities) {
        return Err(Error::Format(
            "manifest.json required_features is not sorted and unique".into(),
        ));
    }
    if let Some(unsupported) = required_capabilities
        .iter()
        .find(|capability| !SUPPORTED_CAPABILITIES.contains(&capability.as_str()))
    {
        return Err(Error::Format(format!(
            "manifest.json requires `{unsupported}`, which the node does not implement"
        )));
    }
    let ruleset_compatibility = strings(&ruleset, "ruleset_compatibility", "ruleset file")?;
    if ruleset_compatibility.len() != 1 {
        return Err(Error::Format(
            "the ruleset file must name exactly one ruleset".into(),
        ));
    }
    let mut provenance_summary = text(&lock, "revision_digest_token", "content.lock.json")?;
    for entry in lock["entries"]
        .as_array()
        .ok_or_else(|| Error::Format("content.lock.json has no entries".into()))?
    {
        provenance_summary.push('/');
        provenance_summary.push_str(&text(entry, "package_provenance_digest", "lock entry")?);
    }
    Ok(Identity {
        project_format_version: text(&project, "schema", "project.json")?,
        world_schema_version: text(&manifest, "semantic_schema_version", "manifest.json")?,
        content_revision: text(&project, "project_revision", "project.json")?,
        content_lock_digest: lock_digest,
        min_runtime_version: RUNTIME_VERSION.to_string(),
        required_capabilities,
        ruleset_compatibility,
        provenance_summary,
    })
}

/// The identity file as committed: pretty JSON and a final newline.
fn identity_text(identity: &Identity) -> Result<String, Error> {
    serde_json::to_string_pretty(identity)
        .map(|text| text + "\n")
        .map_err(|e| Error::Format(e.to_string()))
}

/// The squash-stable identity of the compiler inputs: SHA-256 over the domain tag and one
/// `<git blob id> <path>` line per tracked input file, ascending by path, without the pin files.
fn inputs_digest(root: &Path) -> Result<String, Error> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-s", "-z", "--"])
        .args(INPUT_PATHS)
        .output()
        .map_err(|e| Error::Format(format!("git ls-files: {e}")))?;
    if !output.status.success() {
        return Err(Error::Format("git ls-files failed".into()));
    }
    let listing = String::from_utf8(output.stdout)
        .map_err(|_| Error::Format("git ls-files: path is not UTF-8".into()))?;
    let mut lines = BTreeSet::new();
    for row in listing.split('\0').filter(|row| !row.is_empty()) {
        let (meta, path) = row
            .split_once('\t')
            .ok_or_else(|| Error::Format("git ls-files: malformed row".into()))?;
        let blob = meta
            .split(' ')
            .nth(1)
            .ok_or_else(|| Error::Format("git ls-files: malformed row".into()))?;
        let in_pins = path
            .strip_prefix(PINS)
            .and_then(|rest| rest.strip_prefix('/'))
            .is_some_and(|name| !name.contains('/') && is_pin_file(name));
        if !in_pins {
            lines.insert((path.to_owned(), blob.to_owned()));
        }
    }
    let mut hash = Sha256::new();
    hash.update(INPUTS_DOMAIN.as_bytes());
    hash.update(b"\n");
    for (path, blob) in &lines {
        hash.update(format!("{blob} {path}\n"));
    }
    Ok(hex(&hash.finalize()))
}

/// Compiles the World Project of `root` as a non-production bundle, twice, and returns its bytes.
fn build_twice(root: &Path, identity: &Identity) -> Result<Vec<u8>, Error> {
    let project = project(root)?;
    let (registry, palette) = registry(root, &project)?;
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: identity.clone(),
        world: world_extent(root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: project::draft_areas(&project.index)?,
        families: &project.families,
    };
    let first = compile::compile(&input, &registry)?;
    let second = compile::compile(&input, &registry)?;
    if first.digest != second.digest || first.bytes != second.bytes {
        return Err(Error::Format(
            "two builds from one tree gave different bundles".into(),
        ));
    }
    equivalence(&input, &registry, &first.bytes)?;
    Ok(first.bytes)
}

/// `entry_start` must be a walkable, non-blocking base cell inside the World bounds: the first
/// ground of its top-level entries is a walkable Terrain ground and none of them is a wall.
fn check_entry_start(bytes: &[u8], at: EntryStart) -> Result<(), Error> {
    let bad = |what: &str| Error::Format(format!("entry_start: {what}"));
    let mut found = None;
    let visited = bundle::visit(bytes, |manifest: &Manifest, sector| {
        if sector.floor == at.floor
            && sector.sx == at.x / oteryn_world_bundle_compiler::sector::SECTOR_SIZE
            && sector.sy == at.y / oteryn_world_bundle_compiler::sector::SECTOR_SIZE
            && let Some(tile) = sector.tiles.iter().find(|t| (t.x, t.y) == (at.x, at.y))
        {
            let (mut ground, mut wall) = (None, false);
            for item in tile.items.iter().filter(|item| item.depth == 0) {
                let terrain = manifest.palette[item.palette as usize].terrain;
                if ground.is_none()
                    && let Some(t) = terrain.filter(|t| t.kind == TerrainKind::Ground)
                {
                    ground = t.walkable;
                }
                wall |= terrain.is_some_and(|t| t.kind == TerrainKind::Wall);
            }
            found = Some((ground == Some(true), wall));
        }
        Ok(())
    })?;
    if !visited.manifest.world.contains(at.x, at.y, at.floor) {
        return Err(bad("outside the World bounds"));
    }
    match found {
        None => Err(bad("the bundle has no tile there")),
        Some((false, _)) => Err(bad("the cell has no walkable ground")),
        Some((true, true)) => Err(bad("the cell holds a blocking wall")),
        Some((true, false)) => Ok(()),
    }
}

fn run_derive_identity(root: &Path, slug: &str) -> Result<String, Error> {
    identity_text(&derive_identity(root, slug)?)
}

/// Checks every pin and writes each bundle to `out`. Returns the JSON summary.
fn run_pin_check(root: &Path, out: &Path) -> Result<Value, Error> {
    let io = |e: std::io::Error| Error::Format(format!("{}: {e}", out.display()));
    let pins = root.join(PINS);
    let mut slugs: Vec<String> = fs::read_dir(&pins)
        .map_err(|e| Error::Format(format!("{}: {e}", pins.display())))?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| is_pin_file(name))
        .map(|name| name.trim_end_matches(".json").to_owned())
        .collect();
    slugs.sort();
    if slugs.is_empty() {
        return Err(Error::Format(
            "no World pin under content/world/pins".into(),
        ));
    }
    let tree = inputs_digest(root)?;
    fs::create_dir_all(out).map_err(io)?;
    let (mut reports, mut names) = (Vec::new(), Vec::new());
    for slug in slugs {
        let fail = |what: String| Error::Format(format!("pin {slug}: {what}"));
        let pin: Pin = serde_json::from_slice(&read(&pins.join(format!("{slug}.json")))?)
            .map_err(|e| fail(e.to_string()))?;
        if pin.schema != PIN_SCHEMA {
            return Err(fail(format!("schema is not {PIN_SCHEMA}")));
        }
        if pin.inputs_digest != tree {
            return Err(fail(format!(
                "inputs_digest {} differs from the tree's {tree}",
                pin.inputs_digest
            )));
        }
        let identity = derive_identity(root, &slug)?;
        let committed = fs::read_to_string(pins.join(format!("{slug}.identity.json")))
            .map_err(|e| fail(format!("identity file: {e}")))?;
        if committed != identity_text(&identity)? {
            return Err(fail(
                "the identity file differs from derive-identity".into(),
            ));
        }
        if (
            &pin.project_format_version,
            &pin.world_schema_version,
            &pin.content_revision,
        ) != (
            &identity.project_format_version,
            &identity.world_schema_version,
            &identity.content_revision,
        ) {
            return Err(fail("a revision differs from the identity file".into()));
        }
        if pin.production {
            return Err(fail(
                "a production pin names a non-production build (ADR-0021 §4.2)".into(),
            ));
        }
        let at = pin.entry_start;
        if !world_extent(root)?.contains(at.x, at.y, at.floor) {
            return Err(Error::Format(
                "entry_start: outside the World bounds".into(),
            ));
        }
        let bytes = build_twice(root, &identity)?;
        let digest = hex(&bytes[bytes.len() - 32..]);
        if pin.digest != digest {
            return Err(fail(format!(
                "digest {} is not the tree's {digest}",
                pin.digest
            )));
        }
        check_entry_start(&bytes, pin.entry_start)?;
        fs::write(out.join(format!("{slug}-{digest}.otwb")), &bytes).map_err(io)?;
        names.push(digest.clone());
        reports.push(json!({"world": slug, "digest": digest, "bytes": bytes.len()}));
    }
    fs::write(
        out.join("artifact-name.txt"),
        format!("world-bundle-{}\n", names.join("-")),
    )
    .map_err(io)?;
    Ok(json!({"inputs_digest": tree, "pins": reports}))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let result = match args.as_slice() {
        [_, command, root] if command == "parity" => run_parity(Path::new(root)),
        [_, command, root, identity, out, class] if command == "compile" => {
            run_compile(Path::new(root), Path::new(identity), Path::new(out), class)
        }
        [_, command, root, slug] if command == "derive-identity" => {
            return match run_derive_identity(Path::new(root), slug) {
                Ok(identity) => {
                    print!("{identity}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::FAILURE
                }
            };
        }
        [_, command, root, out] if command == "pin-check" => {
            run_pin_check(Path::new(root), Path::new(out))
        }
        _ => {
            eprintln!(
                "usage: oteryn-world-bundle-compiler parity <repository root>\n       \
                 oteryn-world-bundle-compiler compile <repository root> <identity.json> <out> \
                 <production|non-production>\n       \
                 oteryn-world-bundle-compiler derive-identity <repository root> <world slug>\n       \
                 oteryn-world-bundle-compiler pin-check <repository root> <out directory>"
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
