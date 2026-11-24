//! Compile the shipped incantations that accept parameters for action-button captions.
use std::{collections::BTreeSet, fs, path::Path};

pub(crate) fn generate(root: &Path, out: &Path) -> Result<(), String> {
    let path = root.join("../../content/abilities/definitions/player-spells.json");
    println!("cargo:rerun-if-changed={}", path.display());
    let catalog: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let mut words = BTreeSet::new();
    for bundle in catalog["bundles"]
        .as_array()
        .ok_or("missing spell bundles")?
    {
        let spell = &bundle["bundle"]["spell"];
        if spell["carrier"] == "instant"
            && spell["targeting"]["parameter"]
                .as_str()
                .ok_or("missing spell parameter kind")?
                != "none"
        {
            words.insert(
                spell["words"]
                    .as_str()
                    .ok_or("missing incantation")?
                    .to_ascii_lowercase(),
            );
        }
    }
    let words: Vec<_> = words.into_iter().collect();
    fs::write(out.join("spell_parameters.rs"), format!("&{words:?}")).map_err(|e| e.to_string())
}
