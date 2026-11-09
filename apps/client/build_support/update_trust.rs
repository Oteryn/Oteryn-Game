//! Only reviewed build inputs may supply a TUF bootstrap root; no network bootstrap.
pub fn emit() -> Result<(), String> {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").map_err(|e| e.to_string())?);
    for channel in ["STABLE", "BETA"] {
        let key = format!("OTERYN_CLIENT_TUF_ROOT_{channel}");
        println!("cargo:rerun-if-env-changed={key}");
        let bytes = match std::env::var(&key) {
            Ok(path) => {
                println!("cargo:rerun-if-changed={path}");
                let data = std::fs::read(path).map_err(|_| format!("Cannot read {key}"))?;
                if data.is_empty() || data.len() > 1_048_576 {
                    return Err(format!("Invalid {key} size"));
                }
                serde_json::from_slice::<serde_json::Value>(&data)
                    .map_err(|_| format!("Invalid {key} JSON"))?;
                data
            }
            Err(std::env::VarError::NotPresent) => Vec::new(),
            Err(_) => return Err(format!("Invalid {key}")),
        };
        std::fs::write(
            out.join(format!("update-root-{}.json", channel.to_lowercase())),
            bytes,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
