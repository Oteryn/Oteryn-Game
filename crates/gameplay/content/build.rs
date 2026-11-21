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
