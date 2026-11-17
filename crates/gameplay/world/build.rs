//! Derive movement admission's fallback from the source appearance catalogue.
//! The binary never needs a source checkout at runtime.
use std::{collections::BTreeMap, error::Error, fmt::Write, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let root =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?).join("../../../content/world/objects");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut flags = BTreeMap::new();
    for path in std::fs::read_dir(root)? {
        let path = path?.path();
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("objects-") && name.ends_with(".json"))
        {
            continue;
        }
        let document: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        let records = document["records"]
            .as_array()
            .ok_or("world object records")?;
        for record in records {
            let field = &record["collision"]["blocks_movement"];
            if field["state"] != "KNOWN" {
                continue;
            }
            let id = record["provenance"]["source_item_id"]
                .as_u64()
                .ok_or("world object source id")?;
            let blocks = field["value"]
                .as_bool()
                .ok_or("world object movement flag")?;
            if flags.insert(u32::try_from(id)?, blocks).is_some() {
                return Err("duplicate world object source id".into());
            }
        }
    }
    if flags.is_empty() {
        return Err("empty world object collision catalogue".into());
    }
    let mut output = String::from("const CATALOGUE_MOVEMENT: &[(u32, bool)] = &[\n");
    for (id, blocks) in flags {
        writeln!(output, "({id}, {blocks}),")?;
    }
    output.push_str("];\n");
    std::fs::write(
        PathBuf::from(std::env::var("OUT_DIR")?).join("catalogue_movement.rs"),
        output,
    )?;
    Ok(())
}
