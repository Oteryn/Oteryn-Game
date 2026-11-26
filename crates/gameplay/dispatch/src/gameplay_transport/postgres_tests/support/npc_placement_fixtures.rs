//! Placement assertions follow the published World index across shard boundaries.
use super::TestResult;
use serde_json::Value;
use std::path::Path;

pub(super) fn published(npc: &str) -> TestResult<Option<Value>> {
    let repo = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.."));
    let index: Value = serde_json::from_slice(&std::fs::read(
        repo.join("content/world/npc-placements/index.json"),
    )?)?;
    let mut placement = None;
    for shard in index["shards"].as_array().ok_or("placement shards")? {
        let shard = shard.as_str().ok_or("placement shard path")?;
        let document: Value = serde_json::from_slice(&std::fs::read(repo.join(shard))?)?;
        for record in document["records"].as_array().ok_or("placement records")? {
            let declaration = &record["declaration"];
            if declaration["npc"] == npc {
                assert!(placement.is_none(), "expected one published placement for {npc}");
                placement = Some(declaration.clone());
            }
        }
    }
    Ok(placement)
}
