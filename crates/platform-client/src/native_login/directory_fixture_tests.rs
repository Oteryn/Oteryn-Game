//! client-directory-v1 (Oteryn-Platform 269f2ae0): the four accepted vectors, byte-exact against
//! their manifest, through the parser and the real request path (bearer and 401).
use super::*;
use crate::{DirectoryPvpType, PlatformClient, parse_directory};
use sha2::{Digest, Sha256};

fn fixture_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/contracts/fixtures/client-directory-v1")
}

fn manifest() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(&std::fs::read(
        fixture_root().join("manifest.json"),
    )?)?)
}

/// The vector bytes, checked against the manifest's size and sha256.
fn vector(name: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let manifest = manifest()?;
    let entry = manifest["files"]
        .as_array()
        .ok_or("files")?
        .iter()
        .find(|entry| entry["file"] == name)
        .ok_or("vector not in manifest")?;
    let bytes = std::fs::read(fixture_root().join(name))?;
    assert_eq!(bytes.len() as u64, entry["bytes"].as_u64().ok_or("bytes")?);
    let digest = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(digest, entry["sha256"].as_str().ok_or("sha256")?);
    Ok(bytes)
}

fn runtime() -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
}

fn directory_client(server: &MockServer) -> Result<PlatformClient, PlatformClientError> {
    PlatformClient::new(PlatformClientConfig::new(&server.base)?)
}

#[test]
fn directory_anonymous_is_accepted_without_a_bearer() -> TestResult {
    let body = String::from_utf8(vector("directory-anonymous.json")?)?;
    let runtime = runtime()?;
    let server = spawn_server(&runtime, vec![(200, "", body)])?;
    let directory =
        runtime.block_on(directory_client(&server)?.fetch_directory(CancellationToken::new()))?;
    assert_eq!(directory.epoch.get(), 7);
    let world = &directory.worlds[0];
    assert_eq!(world.display_name.as_str(), "Aurora");
    assert!(world.characters.is_empty());
    let types: Vec<_> = world.channels.iter().map(|c| c.pvp_type).collect();
    assert_eq!(
        types,
        [DirectoryPvpType::OptionalPvp, DirectoryPvpType::OpenPvp]
    );
    assert!(
        world
            .channels
            .iter()
            .all(|c| c.region.as_str() == "eu-central")
    );
    let requests = server.requests.lock().map_err(|_| "requests")?;
    assert!(requests[0].starts_with("GET /v1/client/directory "));
    assert!(!requests[0].to_ascii_lowercase().contains("authorization:"));
    Ok(())
}

#[test]
fn directory_owner_is_accepted_with_the_bearer_and_a_refused_one_is_unauthenticated() -> TestResult
{
    let manifest = manifest()?;
    let bearer = manifest["synthetic_access_token"]
        .as_str()
        .ok_or("token")?
        .to_owned();
    let body = String::from_utf8(vector("directory-owner.json")?)?;
    let runtime = runtime()?;
    let server = spawn_server(
        &runtime,
        vec![
            (200, "", body),
            (401, "", r#"{"error":"unauthenticated"}"#.into()),
        ],
    )?;
    let client = directory_client(&server)?;
    let token = AccessToken(SecretText(bearer.clone()));
    let directory =
        runtime.block_on(client.fetch_directory_as(Some(&token), CancellationToken::new()))?;
    let names: Vec<_> = directory.worlds[0]
        .characters
        .iter()
        .map(|c| c.display_name.as_str().to_owned())
        .collect();
    assert_eq!(names, ["Aldric", "Brenna"]);
    assert_eq!(
        runtime
            .block_on(client.fetch_directory_as(Some(&token), CancellationToken::new()))
            .err(),
        Some(PlatformClientError::Unauthenticated)
    );
    let requests = server.requests.lock().map_err(|_| "requests")?;
    assert!(
        requests[0]
            .lines()
            .any(|line| line.eq_ignore_ascii_case(&format!("authorization: Bearer {bearer}")))
    );
    Ok(())
}

#[test]
fn directory_rejected_route_key_is_a_forbidden_gameplay_field() -> TestResult {
    assert_eq!(
        parse_directory(&vector("directory-rejected-route-key.json")?).err(),
        Some(PlatformClientError::ForbiddenGameplayField)
    );
    Ok(())
}

#[test]
fn directory_unknown_pvp_type_is_an_invalid_payload() -> TestResult {
    assert_eq!(
        parse_directory(&vector("directory-unknown-pvp-type.json")?).err(),
        Some(PlatformClientError::InvalidPayload)
    );
    Ok(())
}

#[test]
fn directory_region_over_32_bytes_is_an_invalid_payload() -> TestResult {
    let body = String::from_utf8(vector("directory-anonymous.json")?)?.replacen(
        "eu-central",
        &"r".repeat(33),
        1,
    );
    assert_eq!(
        parse_directory(body.as_bytes()).err(),
        Some(PlatformClientError::InvalidPayload)
    );
    Ok(())
}
