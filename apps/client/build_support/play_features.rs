//! Temporary app adapters for feature behavior that still consumes the app's map/scene.
//! Each play.json mounts concern modules and registers typed pushed-event reducers.
//! The generated match stays private in OUT_DIR; there is no ordered source registry.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

fn read(path: &Path) -> Result<Value, String> {
    serde_json::from_str(&fs::read_to_string(path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .enumerate()
            .all(|(i, b)| b.is_ascii_alphabetic() || b == b'_' || (i > 0 && b.is_ascii_digit()))
}

pub(crate) fn generate(features: &Path, root: &Path) -> Result<(String, String), String> {
    let mut mechanics = BTreeSet::new();
    for entry in
        fs::read_dir(root.join("crates/protocol-oteryn/capabilities")).map_err(|e| e.to_string())?
    {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let fact = read(&path)?;
            let id = fact["id"].as_u64().ok_or("mechanic id")?;
            let name = fact["name"].as_str().ok_or("mechanic name")?;
            let slug = name.rsplit_once("_V").map_or(name, |(slug, _)| slug);
            mechanics.insert(format!(
                "{id:04}-{}",
                slug.to_ascii_lowercase().replace('_', "-")
            ));
        }
    }
    let mut fragments = BTreeMap::new();
    for entry in fs::read_dir(features).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.join("play.json").is_file() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("feature name")?;
            let local = name.strip_prefix("local-").is_some_and(|slug| {
                !slug.is_empty()
                    && slug
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            });
            if !mechanics.contains(name) && !local {
                return Err(format!("unknown play mechanic {name}"));
            }
            fragments.insert(name.to_owned(), path);
        }
    }
    let mut rust = String::from("// Generated; do not commit.\n");
    let mut tests = String::from("// Generated; do not commit.\n");
    let mut events = BTreeMap::new();
    for (name, path) in fragments {
        let fragment = read(&path.join("play.json"))?;
        if fragment.as_object().is_none_or(|o| o.len() != 2) {
            return Err("play fragment requires only concerns and events".into());
        }
        let concerns = fragment["concerns"].as_array().ok_or("concerns array")?;
        let module = format!("feature_{}", name.replace('-', "_"));
        rust.push_str(&format!("pub(crate) mod {module} {{\n"));
        let mut seen = BTreeSet::new();
        for concern in concerns {
            let concern = concern.as_str().ok_or("concern name")?;
            if !["state", "protocol", "view", "input", "settings"].contains(&concern)
                || !seen.insert(concern)
            {
                return Err(format!("invalid/duplicate play concern {concern}"));
            }
            let source = path
                .join(format!("{concern}.rs"))
                .canonicalize()
                .map_err(|e| e.to_string())?;
            if !source.starts_with(path.canonicalize().map_err(|e| e.to_string())?) {
                return Err("play concern escapes feature tree".into());
            }
            rust.push_str(&format!(
                "#[path = {:?}] pub(crate) mod {concern};\n",
                source.to_str().ok_or("source path")?
            ));
        }
        rust.push_str("}\n");
        let test_path = path.join("tests/play.rs");
        if test_path.is_file() {
            let test_path = test_path.canonicalize().map_err(|e| e.to_string())?;
            tests.push_str(&format!(
                "mod {module} {{ include!({:?}); }}\n",
                test_path.to_str().ok_or("test path")?
            ));
        }
        for (event, handler) in fragment["events"].as_object().ok_or("events object")? {
            let handler = handler.as_str().ok_or("event handler")?;
            if !identifier(event) || !identifier(handler) || !seen.contains("protocol") {
                return Err("invalid play event/handler or missing protocol concern".into());
            }
            if events
                .insert(event.to_owned(), format!("{module}::protocol::{handler}"))
                .is_some()
            {
                return Err(format!("duplicate play event {event}"));
            }
        }
    }
    rust.push_str("fn coalesce_pushed(pushed: &mut Pushed, events: Vec<SessionEvent>) {\nfor event in events {\nmatch event {\n");
    for (event, handler) in events {
        rust.push_str(&format!(
            "SessionEvent::{event}(delta) => {handler}(pushed, delta),\n"
        ));
    }
    rust.push_str("_ => {}\n}\n}\n}\n");
    Ok((rust, tests))
}
