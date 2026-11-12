use oteryn_platform_client::device_sessions::DeviceCredential;
#[test]
fn device_credentials_are_canonical_and_redacted() {
    assert!(DeviceCredential::parse("otd1.bad").is_err());
}

#[test]
fn accepted_device_fixtures_and_ed25519_proofs_are_exact() -> Result<(), Box<dyn std::error::Error>>
{
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use oteryn_platform_client::device_sessions::{DeviceSession, SessionKey, vault_key};
    use sha2::{Digest, Sha256};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/contracts/fixtures/remembered-device-v1");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json"))?)?;
    for entry in manifest["files"].as_array().ok_or("files")? {
        let bytes = std::fs::read(root.join(entry["file"].as_str().ok_or("path")?))?;
        assert_eq!(bytes.len() as u64, entry["bytes"].as_u64().ok_or("bytes")?);
        let digest = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        assert_eq!(digest, entry["sha256"].as_str().ok_or("digest")?);
    }
    for class in ["session", "remembered"] {
        let body: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join(format!("{class}-enroll.json")))?)?;
        let input = std::fs::read(root.join(format!("{class}-proof-input.json")))?;
        let key: [u8; 32] = URL_SAFE_NO_PAD
            .decode(body["device_public_key"].as_str().ok_or("key")?)?
            .try_into()
            .map_err(|_| "key length")?;
        let signature = ed25519_dalek::Signature::from_slice(
            &URL_SAFE_NO_PAD.decode(body["enrollment_proof"].as_str().ok_or("proof")?)?,
        )?;
        ed25519_dalek::VerifyingKey::from_bytes(&key)?.verify_strict(&input, &signature)?;
    }
    let session = DeviceSession::from_vault(
        &std::fs::read(root.join("enroll-response.json"))?,
        1794176000,
    )?;
    assert!(!format!("{session:?}").contains(session.device_credential.expose()));
    assert!(DeviceSession::from_vault(&session.vault_bytes()?, session.idle_expires_at).is_err());
    let key = SessionKey::generate()?;
    let body = key.enrollment("test-bearer", "019a5f4a-1000-7000-8000-000000000001")?;
    assert_eq!(body["remember_device"], false);
    assert_eq!(body.as_object().ok_or("body")?.len(), 4);
    let public = body["device_public_key"]
        .as_str()
        .ok_or("generated public key")?;
    let input = serde_json::json!({"access_token_sha256":Sha256::digest(b"test-bearer").iter().map(|b|format!("{b:02x}")).collect::<String>(),"client_id":"019a5f4a-1000-7000-8000-000000000001","persistence":"session","public_key":public,"purpose":"oteryn-native-device-enroll-v1"});
    let key: [u8; 32] = URL_SAFE_NO_PAD
        .decode(public)?
        .try_into()
        .map_err(|_| "key")?;
    let signature = ed25519_dalek::Signature::from_slice(
        &URL_SAFE_NO_PAD.decode(body["enrollment_proof"].as_str().ok_or("generated proof")?)?,
    )?;
    ed25519_dalek::VerifyingKey::from_bytes(&key)?
        .verify_strict(&serde_json::to_vec(&input)?, &signature)?;
    assert_ne!(
        vault_key(
            "https://account.oteryn.com",
            "019a5f4a-1000-7000-8000-000000000001"
        )?,
        vault_key(
            "https://other.oteryn.com",
            "019a5f4a-1000-7000-8000-000000000001"
        )?
    );
    Ok(())
}
