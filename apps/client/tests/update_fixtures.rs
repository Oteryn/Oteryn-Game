use sha2::{Digest, Sha256};
#[test]
fn platform_update_fixture_manifest_preserves_every_signed_byte()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/contracts/fixtures/client-update-origin-v1");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json"))?)?;
    for entry in manifest["files"].as_array().ok_or("manifest files")? {
        let bytes = std::fs::read(root.join(entry["path"].as_str().ok_or("fixture path")?))?;
        assert_eq!(
            bytes.len() as u64,
            entry["bytes"].as_u64().ok_or("fixture length")?
        );
        assert_eq!(
            Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            entry["sha256"].as_str().ok_or("fixture digest")?
        );
    }
    Ok(())
}
