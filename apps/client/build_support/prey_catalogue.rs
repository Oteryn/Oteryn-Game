//! Bind shipped prey species to the client-safe appearance definitions of the same build.
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::Path};

fn read(path: &Path) -> Result<Value, String> {
    println!("cargo:rerun-if-changed={}", path.display());
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn emit(root: &Path, out: &Path) -> Result<(), String> {
    let roster = read(&root.join("content/creatures/prey-tiers.json"))?;
    let index = read(&root.join("content/presentations/definitions/index.json"))?;
    let mut art = BTreeMap::new();
    for shard in index["shards"].as_array().ok_or("presentation shards")? {
        let data = read(&root.join(shard.as_str().ok_or("presentation path")?))?;
        for record in data["records"].as_array().ok_or("presentation records")? {
            let definition = &record["definition"];
            if definition["client_projection"] != "ClientSafe" {
                continue;
            }
            let Some(key) = definition["identity"]["key"]
                .as_str()
                .and_then(|s| s.strip_prefix("oteryn:presentation.creature."))
            else {
                continue;
            };
            if let Some(binding) = record["authoring"]["profile"]["asset_binding"].as_str() {
                art.insert(format!("oteryn:creature.{key}"), binding.to_owned());
            }
        }
    }
    let mut species = BTreeMap::new();
    for record in roster["records"].as_array().ok_or("prey species")? {
        let key = record["creature"].as_str().ok_or("prey key")?;
        let name = record["display_name"].as_str().ok_or("prey name")?;
        let binding = art
            .get(key)
            .ok_or_else(|| format!("missing prey artwork for {key}"))?;
        let (kind, id) = binding.split_once('/').ok_or("prey artwork id")?;
        let object = match kind {
            "canary.appearance:outfit" => false,
            "canary.appearance:object" => true,
            _ => return Err(format!("unknown appearance binding {binding}")),
        };
        let id = id
            .parse::<std::num::NonZeroU32>()
            .map_err(|e| e.to_string())?;
        if species
            .insert(
                key,
                format!("Species {{key:{key:?}, name:{name:?}, object:{object}, look:{id}}},\n"),
            )
            .is_some()
        {
            return Err(format!("duplicate prey species {key}"));
        }
    }
    let source = format!("&[\n{}]", species.into_values().collect::<String>());
    fs::write(out.join("prey_species.rs"), source).map_err(|e| e.to_string())
}
