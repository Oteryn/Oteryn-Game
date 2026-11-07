//! Look up exact source placements through the generated family index as scope grows.
use super::TestResult;
pub(super) fn declaration(key: &str) -> TestResult<serde_json::Value> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/npc-placements/index.json"),
    )?)?;
    let mut found = None;
    for path in index["shards"].as_array().ok_or("NPC placement shards")? {
        let doc: serde_json::Value = serde_json::from_slice(&std::fs::read(
            root.join(path.as_str().ok_or("NPC placement path")?),
        )?)?;
        for row in doc["records"].as_array().ok_or("NPC placement records")? {
            let declaration = &row["declaration"];
            if declaration["npc"].as_str() == Some(key)
                && found.replace(declaration.clone()).is_some()
            {
                return Err("ambiguous source NPC placement".into());
            }
        }
    }
    found.ok_or_else(|| "source NPC placement absent".into())
}
