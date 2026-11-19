//! client-channel-probe-v1 (Oteryn-Platform d2e54f2e, PR #1939): the four list vectors,
//! byte-exact against their manifest, through the parser and the real request path, and the
//! measurement rules against a local edge (warm-up plus three timed requests, any refusal "—").
use super::*;
use crate::PlatformClient;
use crate::channel_probe::{
    ChannelProbe, measure, measure_all, parse_channel_probes, probe_client,
};
use sha2::{Digest, Sha256};

fn fixture_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/contracts/fixtures/client-channel-probe-v1")
}

/// The vector bytes, checked against the manifest's size and sha256, with its expectation.
fn vector(name: &str) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture_root().join("manifest.json"))?)?;
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
    Ok((bytes, entry["expect"].as_str().ok_or("expect")?.to_owned()))
}

fn runtime() -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
}

fn channel(last: u8) -> [u8; 16] {
    let mut id = [
        0x01, 0x9a, 0x5f, 0x4a, 0x40, 0x00, 0x70, 0x00, 0x80, 0, 0, 0, 0, 0, 0, 0,
    ];
    id[15] = last;
    id
}

#[test]
fn every_list_vector_meets_its_manifest_expectation() -> TestResult {
    for name in [
        "probes-ok.json",
        "probes-rejected-route-key.json",
        "probes-rejected-http-url.json",
        "probes-rejected-query.json",
    ] {
        let (bytes, expect) = vector(name)?;
        let outcome = match parse_channel_probes(&bytes) {
            Ok(_) => "accept".to_owned(),
            Err(PlatformClientError::ForbiddenGameplayField) => {
                "reject:ForbiddenGameplayField".to_owned()
            }
            Err(PlatformClientError::InvalidPayload) => "reject:InvalidPayload".to_owned(),
            Err(other) => format!("reject:{other:?}"),
        };
        assert_eq!(outcome, expect, "{name}");
    }
    let (ok, _) = vector("probes-ok.json")?;
    let list = parse_channel_probes(&ok)?;
    assert_eq!(list.epoch, 7);
    assert_eq!(list.probes.len(), 2);
    assert_eq!(list.probes[0].channel_id, channel(1));
    assert_eq!(
        list.probes[1].probe_url.as_str(),
        "https://probe-eu2.example.invalid/v1/probe"
    );
    // The edge's empty 204 body is one accepted sample.
    assert!(vector("probe-target-204.body")?.0.is_empty());
    Ok(())
}

#[test]
fn a_repeated_channel_a_port_or_credentials_are_invalid() {
    let one = |url: &str, second: &str| {
        format!(
            r#"{{"epoch":1,"probes":[{{"channel_id":"019a5f4a-4000-7000-8000-000000000001","probe_url":"{url}"}},{{"channel_id":"{second}","probe_url":"https://b.example.invalid/v1/probe"}}]}}"#
        )
    };
    let second = "019a5f4a-4000-7000-8000-000000000002";
    let first = "019a5f4a-4000-7000-8000-000000000001";
    assert!(
        parse_channel_probes(one("https://a.example.invalid/v1/probe", second).as_bytes()).is_ok()
    );
    for (url, other) in [
        ("https://a.example.invalid/v1/probe", first),
        ("https://a.example.invalid:8443/v1/probe", second),
        ("https://user@a.example.invalid/v1/probe", second),
        ("https://a.example.invalid/v1/probe#x", second),
    ] {
        assert_eq!(
            parse_channel_probes(one(url, other).as_bytes()),
            Err(PlatformClientError::InvalidPayload),
            "{url} {other}"
        );
    }
    let long = format!("https://a.example.invalid/{}", "p".repeat(240));
    assert_eq!(
        parse_channel_probes(one(&long, second).as_bytes()),
        Err(PlatformClientError::InvalidPayload)
    );
}

#[test]
fn the_list_is_fetched_from_its_route_without_a_credential() -> TestResult {
    let (body, _) = vector("probes-ok.json")?;
    let runtime = runtime()?;
    let server = spawn_server(&runtime, vec![(200, "", String::from_utf8(body)?)])?;
    let client = PlatformClient::new(PlatformClientConfig::new(&server.base)?)?;
    let list = runtime.block_on(client.fetch_channel_probes(CancellationToken::new()))?;
    assert_eq!(list.probes.len(), 2);
    let requests = server.requests.lock().map_err(|_| "requests")?;
    assert!(requests[0].starts_with("GET /v1/client/channel-probes HTTP/1.1"));
    assert!(!requests[0].to_ascii_lowercase().contains("authorization:"));
    Ok(())
}

#[test]
fn a_ping_is_the_median_of_three_timed_heads_after_a_warm_up() -> TestResult {
    let runtime = runtime()?;
    let edge = |statuses: &[u16]| {
        spawn_server(
            &runtime,
            statuses
                .iter()
                .map(|status| (*status, "", String::new()))
                .collect(),
        )
    };
    let probe_client = probe_client()?;
    let server = edge(&[204, 204, 204, 204])?;
    let url = reqwest::Url::parse(&format!("{}v1/probe", server.base))?;
    assert!(runtime.block_on(measure(&probe_client, &url)).is_some());
    let requests = server.requests.lock().map_err(|_| "requests")?.clone();
    assert_eq!(requests.len(), 4, "one warm-up and three timed requests");
    assert!(
        requests
            .iter()
            .all(|r| r.starts_with("HEAD /v1/probe HTTP/1.1"))
    );
    assert!(
        requests
            .iter()
            .all(|r| !r.to_ascii_lowercase().contains("cookie:"))
    );
    // A redirect or a rate limit is no measurement.
    for refused in [302, 429] {
        let server = edge(&[204, 204, refused, 204])?;
        let url = reqwest::Url::parse(&format!("{}v1/probe", server.base))?;
        assert_eq!(runtime.block_on(measure(&probe_client, &url)), None);
    }
    // Two Channels on one edge share one measurement.
    let server = edge(&[204, 204, 204, 204])?;
    let url = reqwest::Url::parse(&format!("{}v1/probe", server.base))?;
    let probes = [1, 2].map(|last| ChannelProbe {
        channel_id: channel(last),
        probe_url: url.clone(),
    });
    let pings = runtime.block_on(measure_all(&probe_client, &probes));
    assert_eq!(pings[0].1, pings[1].1);
    assert!(pings[0].1.is_some());
    assert_eq!(server.requests.lock().map_err(|_| "requests")?.len(), 4);
    Ok(())
}
