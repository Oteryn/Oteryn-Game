//! Public display metadata from the exact content manifest used by the server spell loader.
use sha2::{Digest, Sha256};
use std::path::Path;
pub fn generate(root: &Path, out: &Path) -> Result<(), String> {
    for path in [
        "content/spells.manifest.json",
        "content/abilities/definitions/player-spells.json",
    ] {
        println!("cargo:rerun-if-changed={}", root.join(path).display());
    }
    let read = |path: &Path| std::fs::read(path).map_err(|e| e.to_string());
    let manifest: serde_json::Value =
        serde_json::from_slice(&read(&root.join("content/spells.manifest.json"))?)
            .map_err(|e| e.to_string())?;
    let row = &manifest["catalog"];
    let path = row["path"].as_str().ok_or("catalog path missing")?;
    if path != "abilities/definitions/player-spells.json" {
        return Err("Unexpected spell catalog path".into());
    }
    let bytes = read(&root.join("content").join(path))?;
    if Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
        != row["sha256"].as_str().ok_or("catalog digest missing")?
    {
        return Err("Spell metadata digest differs from server manifest".into());
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let mut rust = String::from("pub(super) const PROFESSIONS: &[(&str, &str, &[&str])] = &[\n");
    for row in value["bundles"].as_array().ok_or("spell bundles missing")? {
        let spell = &row["bundle"]["spell"];
        let name = spell["name"].as_str().ok_or("spell name missing")?;
        let words = spell["words"].as_str().unwrap_or("");
        let vocations: Vec<_> = spell["requirements"]["vocations"]
            .as_array()
            .ok_or("vocations missing")?
            .iter()
            .map(|value| value.as_str().ok_or("invalid vocation"))
            .collect::<Result<_, _>>()?;
        rust.push_str(&format!("({name:?}, {words:?}, &{vocations:?}),\n"));
    }
    rust.push_str("];\n");
    std::fs::write(out.join("spell_metadata.rs"), rust).map_err(|e| e.to_string())
}
