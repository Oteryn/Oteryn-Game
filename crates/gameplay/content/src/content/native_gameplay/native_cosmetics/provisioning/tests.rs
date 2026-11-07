use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

#[test]
fn nx5_load_time_cosmetics_matches_the_producer_and_rejects_tampered_inputs()
-> Result<(), Box<dyn std::error::Error>> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let descriptor_path = root.join("content/cosmetics/outfits/native-cosmetics.json");
    let descriptor = std::fs::read(&descriptor_path)?;
    assert!(descriptor.len() < 200_000);
    let pinned = |bytes: Vec<u8>| PinnedGameplayBytes {
        sha256: super::super::super::hex(crate::content::digest::sha256(&bytes)),
        bytes,
    };
    let section = materialize(pinned(descriptor.clone()), &descriptor_path)?;
    assert!(section.bytes.len() > 200_000);
    assert!(super::super::decode(&section.bytes)?.is_some());
    let mut value: serde_json::Value = serde_json::from_slice(&descriptor)?;
    let expected = value["section_sha256"].as_str().ok_or("section pin")?;
    assert_eq!(section.sha256, expected);
    value["section_sha256"] = "0".repeat(64).into();
    assert!(materialize(pinned(serde_json::to_vec(&value)?), &descriptor_path).is_err());
    let mut value: serde_json::Value = serde_json::from_slice(&descriptor)?;
    value["authoring"]["sha256"] = "0".repeat(64).into();
    assert!(materialize(pinned(serde_json::to_vec(&value)?), &descriptor_path).is_err());
    let scratch = std::env::temp_dir().join(format!(
        "nx5-input-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&scratch)?;
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut value: serde_json::Value = serde_json::from_slice(&descriptor)?;
        value["authoring"]["path"] = root
            .join("content/cosmetics/outfits/authoring.json")
            .to_string_lossy()
            .into_owned()
            .into();
        // Duplicate members must be rejected before normalization can erase them.
        let auth = std::fs::read_to_string(root.join("content/cosmetics/outfits/authoring.json"))?;
        let duplicate = auth.replacen('{', "{\"schema\":\"duplicate\",", 1);
        std::fs::write(scratch.join("authoring.json"), &duplicate)?;
        value["authoring"] = serde_json::json!({"path":"authoring.json","sha256":pinned(duplicate.into_bytes()).sha256});
        assert!(matches!(
            materialize(
                pinned(serde_json::to_vec(&value)?),
                &scratch.join("input.json")
            ),
            Err(ContentError::InvalidArtifact("cosmetics authoring JSON"))
        ));
        // A pinned source manifest still cannot name a shard outside its own directory.
        value["authoring"] =
            serde_json::from_slice::<serde_json::Value>(&descriptor)?["authoring"].clone();
        value["authoring"]["path"] = root
            .join("content/cosmetics/outfits/authoring.json")
            .to_string_lossy()
            .into_owned()
            .into();
        let source = b"{\"files\":[{\"path\":\"../escape.json\",\"sha256\":\"unused\"}]}";
        std::fs::write(scratch.join("manifest.json"), source)?;
        value["source_manifest"] =
            serde_json::json!({"path":"manifest.json","sha256":pinned(source.to_vec()).sha256});
        assert!(matches!(
            materialize(
                pinned(serde_json::to_vec(&value)?),
                &scratch.join("input.json")
            ),
            Err(ContentError::InvalidArtifact(
                "cosmetics source shard locator"
            ))
        ));
        Ok(())
    })();
    std::fs::remove_dir_all(scratch)?;
    result
}

#[test]
fn cosmetics_authoring_accepts_the_byte_limit_and_rejects_one_byte_over()
-> Result<(), Box<dyn std::error::Error>> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let descriptor_path = root.join("content/cosmetics/outfits/native-cosmetics.json");
    let mut descriptor: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&descriptor_path)?)?;
    let source = descriptor_path.parent().ok_or("descriptor parent")?.join(
        descriptor["source_manifest"]["path"]
            .as_str()
            .ok_or("source path")?,
    );
    descriptor["source_manifest"]["path"] = source.to_string_lossy().into_owned().into();
    let scratch = std::env::temp_dir().join(format!(
        "cosmetics-boundary-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&scratch)?;
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut authoring = std::fs::read(root.join("content/cosmetics/outfits/authoring.json"))?;
        assert!(authoring.len() < MAX_AUTHORING_BYTES);
        // JSON whitespace exercises the byte boundary without changing the
        // admitted document, its promotions or the materialized section pin.
        authoring.resize(MAX_AUTHORING_BYTES, b' ');
        let path = scratch.join("authoring.json");
        for extra in [false, true] {
            if extra {
                authoring.push(b' ');
            }
            std::fs::write(&path, &authoring)?;
            descriptor["authoring"] = serde_json::json!({"path":"authoring.json",
                "sha256":super::super::super::hex(crate::content::digest::sha256(&authoring))});
            let bytes = serde_json::to_vec(&descriptor)?;
            let pin = PinnedGameplayBytes {
                sha256: super::super::super::hex(crate::content::digest::sha256(&bytes)),
                bytes,
            };
            let actual = materialize(pin, &scratch.join("input.json"));
            if extra {
                assert!(matches!(
                    actual,
                    Err(ContentError::InvalidArtifact(
                        "native gameplay provisioning file bounds"
                    ))
                ));
            } else {
                assert!(actual.is_ok(), "at-limit authoring rejected: {actual:?}");
            }
        }
        Ok(())
    })();
    std::fs::remove_dir_all(scratch)?;
    result
}
