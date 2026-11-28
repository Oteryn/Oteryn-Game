fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Shared promotion adapters need this harness's private artifact types in their tests.
    emit_cargo_directive("rustc-check-cfg", "cfg(oteryn_gameplay_content_unit_tests)");
    emit_cargo_directive("rustc-cfg", "oteryn_gameplay_content_unit_tests");
    let quests = Path::new("../../../content/quests");
    emit_cargo_directive("rerun-if-changed", quests.display());
    emit_cargo_directive("rerun-if-changed", "build.rs");
    let mut keys = BTreeSet::new();
    collect_quest_items(quests, &mut keys)?;
    let destination = Path::new(&std::env::var("OUT_DIR")?).join("quest-loot-items.txt");
    std::fs::write(destination, keys.into_iter().collect::<Vec<_>>().join("\n"))?;
    embed_trade_shards()?;
    Ok(())
}

use std::collections::BTreeSet;
use std::path::Path;

fn emit_cargo_directive(key: &str, value: impl std::fmt::Display) {
    println!("cargo::{key}={value}");
}

fn collect_item_keys(value: &serde_json::Value, keys: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::String(key) if key.starts_with("oteryn:item.") => {
            keys.insert(key.clone());
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_item_keys(value, keys);
            }
        }
        serde_json::Value::Object(values) => {
            for (key, value) in values {
                if key.starts_with("oteryn:item.") {
                    keys.insert(key.clone());
                }
                collect_item_keys(value, keys);
            }
        }
        _ => {}
    }
}

fn collect_quest_items(
    path: &Path,
    keys: &mut BTreeSet<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_quest_items(&path, keys)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            let value = serde_json::from_reader(std::fs::File::open(&path)?)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            collect_item_keys(&value, keys);
        }
    }
    Ok(())
}

fn embed_trade_shards() -> Result<(), Box<dyn std::error::Error>> {
    const TRADE_DIRECTORY: &str = "content/services/trade/";
    let repo = Path::new("../../..");
    let index_path = repo.join(TRADE_DIRECTORY).join("index.json");
    emit_cargo_directive("rerun-if-changed", index_path.display());
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(index_path)?)?;
    if index["schema"] != "OTERYN_FAMILY_INDEX/v1" || index["family"] != "Service.Trade" {
        return Err("invalid trade family index".into());
    }
    let shards = index["shards"].as_array().ok_or("trade index shards")?;
    if shards.is_empty() {
        return Err("trade family index has no shards".into());
    }
    let mut documents = Vec::with_capacity(shards.len());
    let mut names = BTreeSet::new();
    for shard in shards {
        let relative_path = shard.as_str().ok_or("trade index shard path")?;
        let range = relative_path
            .strip_prefix(TRADE_DIRECTORY)
            .and_then(|name| name.strip_prefix("trade-"))
            .and_then(|name| name.strip_suffix(".json"))
            .and_then(|range| range.split_once('-'));
        if !range.is_some_and(|(start, end)| {
            [start, end]
                .into_iter()
                .all(|bound| bound.len() == 5 && bound.bytes().all(|byte| byte.is_ascii_digit()))
        }) || !names.insert(relative_path)
        {
            return Err(format!("invalid or duplicate trade shard path: {relative_path}").into());
        }
        let path = repo.join(relative_path);
        emit_cargo_directive("rerun-if-changed", path.display());
        let document = std::fs::read_to_string(path)?;
        if document.contains('\0') {
            return Err(format!("unexpected NUL in trade shard: {relative_path}").into());
        }
        documents.push(document);
    }
    let output = Path::new(&std::env::var("OUT_DIR")?).join("trade-shards.txt");
    std::fs::write(output, documents.join("\0"))?;
    Ok(())
}
