//! Settings consumes the same feature registrations as the UI architecture checker.
use std::{collections::BTreeMap, fs, path::Path};

pub(crate) fn generate(root: &Path) -> Result<String, String> {
    let mut bindings = BTreeMap::new();
    let features = root.join("crates/client-ui/src/features");
    for feature in fs::read_dir(features).map_err(|e| e.to_string())? {
        let registrations = feature
            .map_err(|e| e.to_string())?
            .path()
            .join("registrations");
        for entry in fs::read_dir(registrations).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            let value: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            if value["kind"] == "keybindings" && value["status"] == "supported" {
                let id = value["id"].as_str().ok_or("binding id")?.to_owned();
                let chord = value["chord"]
                    .as_str()
                    .ok_or("binding chord")?
                    .to_uppercase();
                bindings.insert(id, chord);
            }
        }
    }
    // Presentation's existing physical controls predate UI registrations.
    for (id, chord) in [
        ("audio-mute", "F10"),
        ("audio-down", "F11"),
        ("audio-up", "F12"),
    ] {
        bindings.insert(id.into(), chord.into());
    }
    let mut source = String::from("const REGISTERED_BINDINGS: &[(&str, &str)] = &[\n");
    for (id, chord) in bindings {
        source.push_str(&format!("({id:?}, {chord:?}),\n"));
    }
    source.push_str("];\n");
    // Offline login and sibling rows are not all feature registrations on this checkout.
    source.push_str("const RESERVED_CHORDS: &[&str] = &[\"ENTER\", \"KEYPADENTER\", \"CTRL+SHIFT+G\", \"CTRL+SHIFT+L\", \"CTRL+P\"];\n");
    Ok(source)
}
