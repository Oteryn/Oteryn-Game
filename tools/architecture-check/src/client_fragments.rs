//! Directory-scanned, one-entry JSON registrations, shared with client-ui/build.rs.
//! Like the server capability generator, validation precedes deterministic emission.

#[path = "client_fragments/registrations.rs"]
mod registrations;
#[path = "client_fragments/schema.rs"]
mod schema;

use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub(crate) fn read(path: &Path) -> Result<Value, String> {
    serde_json::from_str(&fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?)
        .map_err(|e| format!("{}: {e}", path.display()))
}

pub(crate) fn listing(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .map(|e| e.map(|e| e.path()).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    for path in &paths {
        if fs::symlink_metadata(path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err(format!(
                "symlink in client registration tree: {}",
                path.display()
            ));
        }
    }
    Ok(paths)
}

pub(crate) fn mechanics(root: &Path) -> Result<BTreeMap<String, Value>, String> {
    let mut facts = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for path in listing(&root.join("crates/protocol-oteryn/capabilities"))? {
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let entry = read(&path)?;
        let id = entry["id"].as_u64().ok_or("capability id")?;
        if !ids.insert(id) {
            return Err(format!("duplicate server mechanic id {id}"));
        }
        let name = entry["name"].as_str().ok_or("capability name")?;
        let slug = match name.rsplit_once("_V") {
            Some((slug, version))
                if !version.is_empty() && version.bytes().all(|b| b.is_ascii_digit()) =>
            {
                slug
            }
            _ => name,
        };
        let key = format!("{id:04}-{}", slug.to_ascii_lowercase().replace('_', "-"));
        if facts.insert(key, entry).is_some() {
            return Err("duplicate mechanic".into());
        }
    }
    Ok(facts)
}

pub(crate) fn safe_path(base: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if path
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
        || relative.contains('\\')
    {
        return Err(format!("non-relative registration path: {relative}"));
    }
    let target = base
        .join(path)
        .canonicalize()
        .map_err(|e| format!("{relative}: {e}"))?;
    if !target.starts_with(base.canonicalize().map_err(|e| e.to_string())?) {
        return Err(format!("registration escapes feature: {relative}"));
    }
    Ok(target)
}

pub(crate) fn local_name(name: &str) -> bool {
    name.strip_prefix("local-").is_some_and(|slug| {
        !slug.is_empty()
            && slug
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    })
}

pub(crate) fn scan(root: &Path) -> Result<(Vec<Value>, Vec<Value>), String> {
    let facts = mechanics(root)?;
    let schema = read(&root.join("tools/architecture-check/client-fragment.schema.json"))?;
    let features = root.join("crates/client-ui/src/features");
    let mut entries = Vec::new();
    let mut ids = BTreeSet::new();
    let mut checks = registrations::Checks::default();
    let mut coverage = Vec::new();
    for feature in listing(&features)? {
        let name = feature
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("feature name")?;
        if !feature.is_dir() || (!facts.contains_key(name) && !local_name(name)) {
            return Err(format!("unknown client mechanic directory {name}"));
        }
        let mut discovered = Vec::new();
        for path in listing(&feature.join("registrations"))? {
            if path.extension().is_none_or(|e| e != "json") {
                return Err(format!("registration must be JSON: {}", path.display()));
            }
            let entry = read(&path)?;
            schema::validate(&entry, &schema).map_err(|e| format!("{}: {e}", path.display()))?;
            if entry["mechanic"] != name {
                return Err("mechanic/directory mismatch".into());
            }
            let id = entry["id"].as_str().ok_or("registration id")?;
            registrations::identifier(id)?;
            if !ids.insert(id.to_ascii_lowercase()) {
                return Err(format!("duplicate registration id {id}"));
            }
            registrations::validate(
                root,
                &feature,
                &entry,
                &schema,
                facts.get(name),
                &mut checks,
            )?;
            discovered.push(entry);
        }
        if discovered.is_empty() {
            return Err(format!("missing registrations: {name}"));
        }
        validate_layout(&feature, &discovered)?;
        let golden = read(&feature.join("fixtures/registrations.json"))?;
        discovered.sort_by_key(|e| e["id"].as_str().unwrap_or_default().to_owned());
        if golden != json!(discovered) {
            return Err(format!("per-feature registration golden differs: {name}"));
        }
        if let Some(fact) = facts.get(name) {
            let handlers: Vec<_> = discovered
                .iter()
                .filter(|e| e["kind"] == "handlers")
                .collect();
            let declared: BTreeSet<_> = handlers
                .iter()
                .flat_map(|h| h["commands"].as_array().into_iter().flatten())
                .filter_map(Value::as_u64)
                .collect();
            let expected: BTreeSet<_> = fact["commands"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_u64)
                .collect();
            if declared != expected || handlers.is_empty() {
                return Err(format!("missing/extra command handler: {name}"));
            }
            let unsupported = handlers.iter().any(|e| e["status"] != "supported");
            coverage.push(
                json!({"mechanic":name,"status":if unsupported {"unsupported"} else {"supported"}}),
            );
        }
        entries.extend(discovered);
    }
    let covered: BTreeSet<_> = coverage
        .iter()
        .filter_map(|e| e["mechanic"].as_str())
        .collect();
    if covered != facts.keys().map(String::as_str).collect() {
        return Err("mechanic coverage must include every server capability".into());
    }
    let baseline = read(&root.join("tools/architecture-check/client-unsupported-baseline.json"))?;
    let baseline: BTreeSet<_> = baseline
        .as_array()
        .ok_or("unsupported baseline array")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let unsupported: BTreeSet<_> = coverage
        .iter()
        .filter(|e| e["status"] == "unsupported")
        .filter_map(|e| e["mechanic"].as_str())
        .collect();
    if !unsupported.is_subset(&baseline) {
        return Err("unsupported client coverage grew".into());
    }
    if unsupported != baseline {
        return Err("stale unsupported baseline; remove supported/deleted mechanics".into());
    }
    entries.sort_by_key(|e| {
        let name = e["mechanic"].as_str().unwrap_or_default();
        let number = name
            .split_once('-')
            .and_then(|(n, _)| n.parse::<u64>().ok())
            .unwrap_or(u64::MAX);
        (
            number,
            name.to_owned(),
            e["id"].as_str().unwrap_or_default().to_owned(),
        )
    });
    coverage.sort_by_key(|e| {
        e["mechanic"]
            .as_str()
            .and_then(|n| n.split_once('-'))
            .and_then(|(n, _)| n.parse::<u64>().ok())
            .unwrap_or(u64::MAX)
    });
    registrations::parents(&entries)?;
    Ok((entries, coverage))
}

fn validate_layout(feature: &Path, entries: &[Value]) -> Result<(), String> {
    for concern in ["state", "protocol", "view", "input", "settings"] {
        if !feature.join(format!("{concern}.rs")).is_file()
            && !feature.join(concern).is_dir()
            && !entries.iter().any(|e| {
                e["not_applicable"][concern]
                    .as_str()
                    .is_some_and(|r| !r.trim().is_empty())
            })
        {
            return Err(format!("missing concern {concern}: {}", feature.display()));
        }
    }
    for path in listing(feature)? {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("feature path")?;
        let stem = name.strip_suffix(".rs").unwrap_or(name);
        if ![
            "mod",
            "state",
            "protocol",
            "view",
            "input",
            "settings",
            "tests",
            "registrations",
            "fixtures",
        ]
        .contains(&stem)
        {
            return Err(format!("invalid feature concern {name}"));
        }
    }
    Ok(())
}

pub(crate) fn render(entries: &[Value], coverage: &[Value]) -> Result<(String, String), String> {
    let registrations = serde_json::to_string(&entries).map_err(|e| e.to_string())?;
    let report = serde_json::to_string_pretty(&coverage).map_err(|e| e.to_string())?;
    let pages = entries
        .iter()
        .filter(|entry| entry["kind"] == "cyclopedia-pages" && entry["status"] == "supported")
        .map(|entry| {
            let mechanic = entry["mechanic"].as_str().ok_or("cyclopedia mechanic")?;
            let id = entry["id"].as_str().ok_or("cyclopedia page id")?;
            let parent = entry["parent"].as_str().ok_or("cyclopedia page parent")?;
            let entry = entry["entry"].as_str().ok_or("cyclopedia page entry")?;
            Ok(format!(
                "    CyclopediaPageRegistration {{ mechanic: {mechanic:?}, id: {id:?}, parent: {parent:?}, entry: {entry:?} }},\n"
            ))
        })
        .collect::<Result<String, String>>()?;
    let rust = format!(
        "// Generated; do not commit.\nconst _: &str = {registrations:?};\nconst _: &str = {report:?};\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub struct CyclopediaPageRegistration {{\n    pub mechanic: &'static str,\n    pub id: &'static str,\n    pub parent: &'static str,\n    pub entry: &'static str,\n}}\n\nstatic CYCLOPEDIA_PAGES: &[CyclopediaPageRegistration] = &[\n{pages}];\n\n#[must_use]\npub const fn cyclopedia_pages() -> &'static [CyclopediaPageRegistration] {{ CYCLOPEDIA_PAGES }}\n"
    );
    Ok((rust, report))
}
