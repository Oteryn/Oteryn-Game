use super::{read, safe_path, schema};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Default)]
pub(super) struct Checks {
    commands: BTreeSet<u64>,
    bindings: BTreeMap<String, BTreeSet<u64>>,
    settings: BTreeSet<String>,
}

pub(super) fn identifier(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-:/".contains(&b))
    {
        return Err(format!("invalid registration identifier {value}"));
    }
    Ok(())
}

pub(super) fn validate(
    root: &Path,
    feature: &Path,
    entry: &Value,
    schema: &Value,
    fact: Option<&Value>,
    checks: &mut Checks,
) -> Result<(), String> {
    let kind = entry["kind"].as_str().ok_or("kind")?;
    let status = entry["status"].as_str().ok_or("status")?;
    if status != "supported" {
        if entry["reason"].as_str().is_none_or(|r| r.trim().is_empty()) {
            return Err("unsupported/not_applicable registration needs a reason".into());
        }
        if kind != "handlers" && status == "unsupported" {
            return Err("unsupported is a handler status".into());
        }
    } else {
        schema::validate(entry, &schema["$defs"][kind])?;
        if let Some(entry) = entry["entry"].as_str() {
            symbol(feature, entry)?;
        }
    }
    if kind == "handlers" {
        let fact = fact.ok_or("local UI cannot own a server command")?;
        if entry["status"] == "not_applicable" {
            return Err("server handler cannot be not_applicable".into());
        }
        let commands = entry["commands"].as_array().ok_or("handler commands")?;
        if status == "supported" && commands.len() != 1 {
            return Err("one handler fragment per command".into());
        }
        for command in commands {
            let id = command.as_u64().ok_or("command id")?;
            if !checks.commands.insert(id) {
                return Err(format!("duplicate command handler {id}"));
            }
            if !fact["commands"]
                .as_array()
                .is_some_and(|a| a.contains(command))
            {
                return Err(format!("command {id} does not belong to mechanic"));
            }
            if status == "supported" {
                if entry["capability"] != fact["id"] || entry["direction"] != "client_to_server" {
                    return Err("invalid handler direction/capability".into());
                }
                let dir = root.join("docs/contracts/protocol-oteryn/v1/registry/command_types");
                let mut matched = false;
                for path in super::listing(&dir)? {
                    if path.extension().is_some_and(|e| e == "json") {
                        let descriptor = read(&path)?;
                        if descriptor["id"] == *command {
                            if descriptor["capability"] != fact["id"] {
                                return Err("command descriptor capability mismatch".into());
                            }
                            matched = true;
                        }
                    }
                }
                if !matched {
                    return Err(format!("missing command descriptor {id}"));
                }
            }
        }
    }
    if kind == "keybindings" && status == "supported" {
        let context = entry["context"].as_str().ok_or("key context")?;
        identifier(context)?;
        let chord = chord(entry["chord"].as_str().ok_or("key chord")?)?;
        let priority = entry["precedence"].as_u64().unwrap_or(0);
        if priority > 255 {
            return Err("key precedence must fit u8".into());
        }
        let priorities = checks
            .bindings
            .entry(format!("{context}:{chord}"))
            .or_default();
        if !priorities.insert(priority) {
            return Err(format!("default keybinding conflict: {context}:{chord}"));
        }
    }
    if kind == "settings-pages" && status == "supported" {
        if !["boolean", "integer", "number", "string", "object", "array"]
            .iter()
            .any(|t| entry["setting_type"] == *t)
            || entry["version"].as_u64().is_none_or(|v| v == 0)
        {
            return Err("invalid setting type/version".into());
        }
        let key = entry["key"].as_str().ok_or("setting key")?;
        identifier(key)?;
        if !checks.settings.insert(key.to_owned()) {
            return Err(format!("duplicate setting key {key}"));
        }
        // Keep the complete envelope as JSON in generation; never project it onto
        // known keys (including nested objects and future-version values).
        let encoded =
            serde_json::to_string(&entry["settings_envelope"]).map_err(|e| e.to_string())?;
        let decoded: Value = serde_json::from_str(&encoded).map_err(|e| e.to_string())?;
        if decoded != entry["settings_envelope"] {
            return Err("settings envelope lost unknown keys".into());
        }
    }
    if kind == "assets" && status == "supported" {
        let path = safe_path(feature, entry["asset"].as_str().ok_or("asset path")?)?;
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        let digest = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        if entry["digest"] != digest {
            return Err("asset digest mismatch".into());
        }
    }
    Ok(())
}

fn symbol(feature: &Path, entry: &str) -> Result<(), String> {
    let (file, name) = entry
        .split_once("::")
        .ok_or("entry must be relative-file.rs::function")?;
    if !file.ends_with(".rs") {
        return Err("entry must name a Rust source".into());
    }
    let path = safe_path(feature, file)?;
    let source = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let syntax = syn::parse_file(&source).map_err(|e| e.to_string())?;
    if !syntax
        .items
        .iter()
        .any(|item| matches!(item, syn::Item::Fn(f) if f.sig.ident == name))
    {
        return Err(format!("missing Rust entry symbol {entry}"));
    }
    Ok(())
}

fn chord(chord: &str) -> Result<String, String> {
    let mut parts: Vec<_> = chord
        .split('+')
        .map(str::trim)
        .map(str::to_ascii_uppercase)
        .collect();
    if parts.is_empty()
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
    {
        return Err("invalid default key chord".into());
    }
    parts.sort();
    if parts.windows(2).any(|p| p[0] == p[1]) {
        return Err("duplicate key chord component".into());
    }
    let keys = parts
        .iter()
        .filter(|p| !["CTRL", "ALT", "SHIFT", "SUPER"].contains(&p.as_str()))
        .count();
    if keys != 1 {
        return Err("key chord requires exactly one non-modifier key".into());
    }
    Ok(parts.join("+"))
}

pub(super) fn parents(entries: &[Value]) -> Result<(), String> {
    let ids: BTreeSet<_> = entries.iter().filter_map(|e| e["id"].as_str()).collect();
    let mut graph = BTreeMap::new();
    for entry in entries {
        if let Some(parent) = entry["parent"].as_str() {
            if parent != "root" && !ids.contains(parent) {
                return Err(format!("unknown page/panel parent {parent}"));
            }
            graph.insert(entry["id"].as_str().ok_or("id")?, parent);
        }
    }
    for id in graph.keys() {
        let mut seen = BTreeSet::new();
        let mut current = *id;
        while let Some(parent) = graph.get(current) {
            if !seen.insert(current) {
                return Err("page/panel parent cycle".into());
            }
            current = parent;
        }
    }
    Ok(())
}
