//! Assembles the contract registries stored as one file per entry (REGISTRY-SPLIT-1).
//!
//! `PROTOCOL_OTERYN_V1_REGISTRY.json` and `RESOURCE_LIMITS_REGISTRY.json` are a base file plus
//! one fragment file per array entry. This script scans the fragment directories (there is no
//! list file), validates every fragment and writes each aggregate to `OUT_DIR`. The aggregate
//! text is the base with each placeholder line `  "<key>": [],` replaced by the fragments in
//! order, every fragment line indented by four spaces. `tools/contracts/registry_fragments.py`
//! assembles the same text for Python; keep the two in step.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

struct Registry {
    output: &'static str,
    base: &'static str,
    /// Each collection key with its required fields; an empty list means the base's
    /// `required_entry_fields`.
    collections: &'static [(&'static str, &'static [&'static str])],
    /// A file name's numeric prefix is the entry's `id` (else an ordinal and the name is
    /// `<ordinal>-<id>.json`).
    prefix_is_id: bool,
}

const PROTOCOL: Registry = Registry {
    output: "PROTOCOL_OTERYN_V1_REGISTRY.json",
    base: "docs/contracts/protocol-oteryn/v1/registry/registry.json",
    collections: &[
        (
            "capabilities",
            &[
                "id",
                "name",
                "owner_decision",
                "owner",
                "command_types",
                "state_domains",
                "offered",
            ],
        ),
        (
            "command_types",
            &[
                "id",
                "name",
                "owner_decision",
                "payload_schema",
                "result_schema",
                "max_payload_bytes",
                "max_result_payload_bytes",
            ],
        ),
        (
            "state_domains",
            &[
                "id",
                "name",
                "owner_decision",
                "owner",
                "delta_types",
                "snapshot_types",
            ],
        ),
    ],
    prefix_is_id: true,
};

const RESOURCE_LIMITS: Registry = Registry {
    output: "RESOURCE_LIMITS_REGISTRY.json",
    base: "docs/contracts/resource-limits/registry.json",
    collections: &[("entries", &[])],
    prefix_is_id: false,
};

struct Fragment {
    prefix: u64,
    name: String,
    text: String,
}

fn watch(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());
}

fn read(path: &Path) -> Result<String, String> {
    watch(path);
    fs::read_to_string(path)
        .map(|text| text.replace("\r\n", "\n"))
        .map_err(|error| format!("{}: {error}", path.display()))
}

/// The numeric prefix of `<number>-<slug>.json`, with `slug` in `[A-Za-z0-9._-]+`.
fn prefix(name: &str) -> Option<u64> {
    let stem = name.strip_suffix(".json")?;
    let (number, slug) = stem.split_once('-')?;
    let valid_slug = !slug.is_empty()
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte));
    let valid_number = !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit());
    if valid_slug && valid_number {
        number.parse().ok()
    } else {
        None
    }
}

fn fragments(directory: &Path) -> Result<Vec<Fragment>, String> {
    watch(directory);
    let mut fragments = Vec::new();
    let listing = match fs::read_dir(directory) {
        Ok(listing) => listing,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(fragments),
        Err(error) => return Err(format!("{}: {error}", directory.display())),
    };
    for item in listing {
        let item = item.map_err(|error| format!("{}: {error}", directory.display()))?;
        let name = item.file_name().to_string_lossy().into_owned();
        if !name.ends_with(".json") {
            continue;
        }
        let prefix = prefix(&name).ok_or_else(|| {
            format!(
                "{}/{name}: a fragment is named <number>-<slug>.json",
                directory.display()
            )
        })?;
        let text = read(&item.path())?;
        if !text.ends_with('\n') || text.ends_with("\n\n") {
            return Err(format!(
                "{}/{name}: a fragment ends with exactly one newline",
                directory.display()
            ));
        }
        fragments.push(Fragment { prefix, name, text });
    }
    fragments.sort_by(|left, right| (left.prefix, &left.name).cmp(&(right.prefix, &right.name)));
    Ok(fragments)
}

fn check_entry(
    registry: &Registry,
    where_: &str,
    fragment: &Fragment,
    entry: &Value,
    required: &[String],
    seen: &mut BTreeSet<String>,
) -> Result<(), String> {
    let object = entry
        .as_object()
        .ok_or_else(|| format!("{where_}: a fragment is one JSON object"))?;
    let missing: Vec<_> = required
        .iter()
        .filter(|field| !object.contains_key(field.as_str()))
        .collect();
    if !missing.is_empty() {
        return Err(format!("{where_}: missing {missing:?}"));
    }
    let id = if registry.prefix_is_id {
        let id = object
            .get("id")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("{where_}: `id` is an integer"))?;
        if id != fragment.prefix {
            return Err(format!(
                "{where_}: name prefix {} is not id {id}",
                fragment.prefix
            ));
        }
        id.to_string()
    } else {
        let id = object
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{where_}: `id` is a string"))?;
        let ordinal = fragment
            .name
            .split_once('-')
            .map_or("", |(number, _)| number);
        if fragment.name != format!("{ordinal}-{id}.json") {
            return Err(format!("{where_}: a fragment is named <ordinal>-<id>.json"));
        }
        id.to_owned()
    };
    // Case-insensitive, so the tree also checks out on a case-insensitive file system.
    if !seen.insert(id.to_lowercase()) {
        return Err(format!("{where_}: duplicate id {id}"));
    }
    Ok(())
}

fn assemble(root: &Path, registry: &Registry) -> Result<String, String> {
    let base_path = root.join(registry.base);
    let base_text = read(&base_path)?;
    let base: Value =
        serde_json::from_str(&base_text).map_err(|error| format!("{}: {error}", registry.base))?;
    let base_required: Vec<String> = base
        .get("required_entry_fields")
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .filter_map(|field| field.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let directory_of = base_path.parent().unwrap_or(root).to_path_buf();
    let mut lines: Vec<String> = base_text.split('\n').map(str::to_owned).collect();
    for (key, required) in registry.collections {
        if base.get(*key).and_then(Value::as_array).map(Vec::len) != Some(0) {
            return Err(format!(
                "{}: `{key}` is [] (its entries are fragment files)",
                registry.base
            ));
        }
        let required: Vec<String> = if required.is_empty() {
            base_required.clone()
        } else {
            required.iter().map(|field| (*field).to_owned()).collect()
        };
        let open = format!("  \"{key}\": []");
        let hits: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| **line == open || **line == format!("{open},"))
            .map(|(index, _)| index)
            .collect();
        let [index] = hits[..] else {
            return Err(format!(
                "{}: needs exactly one `{open},` line",
                registry.base
            ));
        };
        let directory: PathBuf = directory_of.join(key);
        let fragments = fragments(&directory)?;
        if fragments.is_empty() {
            continue;
        }
        let mut seen = BTreeSet::new();
        let mut bodies = Vec::with_capacity(fragments.len());
        for fragment in &fragments {
            let where_ = format!("{}/{}", directory.display(), fragment.name);
            let entry: Value = serde_json::from_str(&fragment.text)
                .map_err(|error| format!("{where_}: {error}"))?;
            check_entry(registry, &where_, fragment, &entry, &required, &mut seen)?;
            let body: Vec<String> = fragment
                .text
                .trim_end_matches('\n')
                .split('\n')
                .map(|line| format!("    {line}"))
                .collect();
            bodies.push(body.join("\n"));
        }
        let comma = lines
            .get(index)
            .and_then(|line| line.strip_prefix(&open))
            .unwrap_or_default()
            .to_owned();
        lines.splice(
            index..=index,
            [
                format!("  \"{key}\": ["),
                bodies.join(",\n"),
                format!("  ]{comma}"),
            ],
        );
    }
    let aggregate = lines.join("\n");
    let parsed: Value = serde_json::from_str(&aggregate)
        .map_err(|error| format!("{} aggregate: {error}", registry.output))?;
    for (key, _) in registry.collections {
        if parsed.get(*key).and_then(Value::as_array).is_none() {
            return Err(format!(
                "{} aggregate: `{key}` is not an array",
                registry.output
            ));
        }
    }
    Ok(aggregate)
}

fn run() -> Result<(), String> {
    watch(Path::new("build.rs"));
    let manifest = std::env::var("CARGO_MANIFEST_DIR").map_err(|error| error.to_string())?;
    let out = std::env::var("OUT_DIR").map_err(|error| error.to_string())?;
    let root = Path::new(&manifest).join("../..");
    for registry in [&PROTOCOL, &RESOURCE_LIMITS] {
        let aggregate = assemble(&root, registry)?;
        let target = Path::new(&out).join(registry.output);
        fs::write(&target, aggregate).map_err(|error| format!("{}: {error}", target.display()))?;
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
