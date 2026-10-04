//! PREM-E2E-1: the cross-repository end-to-end test, Game half (PREMIUM-DELIVERY-0 §3.1, §12).
//!
//! Platform's shared fixtures for `oteryn.premium_snapshot.v1` are the producer truth. They are
//! vendored unmodified in `tests/fixtures/premium-snapshot-v1/`, pinned to a Platform commit by
//! `PROVENANCE.md`. This test proves that:
//!
//! - the vendored files are the pinned ones (hashes);
//! - both compatibility records agree: the manifest's policy values and wire constants equal
//!   Game's (consumer contract §4; PREMIUM-DELIVERY-0 §4);
//! - Game's request is the fixture request, on the contract's path;
//! - Game accepts every valid snapshot, and refuses every invalid one with the outcome the
//!   contract requires: no fresh authority, as a failed pull (`Malformed`) or a durable semantic
//!   failure (`Unsupported`, §3.1 item 2);
//! - every valid snapshot, served by the in-process mutual-TLS producer on the contract's path,
//!   reaches validation through the production client.
//!
//! The classification of each valid fixture is proven in `premium::tests`
//! (`platform_fixtures_classify_as_the_contract_requires`). The real-endpoint run against PREM-P
//! is part of the activation record, not of this test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

#[allow(dead_code)]
#[path = "../src/premium/test_producer.rs"]
mod test_producer;

use oteryn_game_server::durability::premium_fence::EntitlementState;
use oteryn_game_server::premium::client::{
    MAX_REQUEST_BYTES, PremiumClientConfig, PremiumSnapshotClient, REQUEST_SCHEMA, SNAPSHOT_PATH,
    request_body,
};
use oteryn_game_server::premium::snapshot::{
    MAX_SNAPSHOT_BYTES, SnapshotRejection, canonical_uuid, validate,
};
use oteryn_game_server::premium::{
    MAX_AUTHORITY_LEASE_US, MAX_CLOCK_SKEW_US, PRODUCER_PROFILE, PRODUCT_ID, PRODUCT_VERSION,
    SNAPSHOT_SCHEMA, Surface, SurfacePolicy, surface_policy,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use test_producer::{Reply, TestPki, TestProducer};

const VALID: [&str; 5] = [
    "snapshot.active.json",
    "snapshot.active-clipped.json",
    "snapshot.expired.json",
    "snapshot.revoked.json",
    "snapshot.none.json",
];

/// Game's outcome for each invalid snapshot of the manifest. Both deny benefit ("no fresh
/// authority", PREM-P §8.1); `Unsupported` also records the durable conflict (§3.1 item 2).
const INVALID: [(&str, SnapshotRejection); 20] = {
    use SnapshotRejection::{Malformed, Unsupported};
    [
        ("unknown member", Malformed),
        ("missing member", Malformed),
        ("wrong schema id", Unsupported),
        ("missing producer_profile", Malformed),
        ("wrong producer_profile", Unsupported),
        ("wrong product", Unsupported),
        ("wrong product version", Unsupported),
        ("fractional-second timestamp", Malformed),
        ("offset timestamp", Malformed),
        ("upper-case uuid", Malformed),
        ("UUIDv4 account_id", Malformed),
        ("entitlement_id with non-RFC variant", Malformed),
        ("NONE with entitlement_id", Malformed),
        ("ACTIVE with null interval", Malformed),
        ("lease longer than 3600 s", Unsupported),
        ("cutoff after effective_until", Malformed),
        ("refresh_after not before cutoff", Malformed),
        ("refresh_after not two thirds of lease", Malformed),
        ("ACTIVE although issued at effective_until", Malformed),
        ("revision above 2^53-1", Malformed),
    ]
};

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/premium-snapshot-v1")
}

fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(dir().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn json(name: &str) -> Value {
    serde_json::from_slice(&bytes(name)).unwrap()
}

fn manifest() -> Value {
    json("manifest.json")
}

/// The request the fixtures answer: its AccountId (bytes) and nonce.
fn fixture_request() -> ([u8; 16], String) {
    let request = json("request.valid.json");
    let text = request["account_id"].as_str().unwrap().replace('-', "");
    let mut account = [0u8; 16];
    for (i, byte) in account.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).unwrap();
    }
    (account, request["nonce"].as_str().unwrap().to_owned())
}

#[test]
fn the_vendored_fixtures_are_the_pinned_platform_files() {
    let provenance = String::from_utf8(bytes("PROVENANCE.md")).unwrap();
    assert!(provenance.contains("`71bbe6c5cffc29d430195fa286b3c6941af4abb8`"));
    let mut pinned = Vec::new();
    for line in provenance.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if let [_, file, hash, _] = cells.as_slice()
            && file.ends_with(".json`")
        {
            pinned.push((file.trim_matches('`').to_owned(), hash.trim_matches('`')));
            let digest: String = Sha256::digest(bytes(file.trim_matches('`')))
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            assert_eq!(digest, hash.trim_matches('`'), "{file}");
        }
    }
    let mut on_disk: Vec<String> = std::fs::read_dir(dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".json"))
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = pinned.into_iter().map(|(file, _)| file).collect();
    listed.sort();
    assert_eq!(listed, on_disk, "every vendored file is pinned");
}

#[test]
fn both_compatibility_records_agree() {
    let manifest = manifest();
    assert_eq!(manifest["fixture_set"], SNAPSHOT_SCHEMA);
    assert_eq!(manifest["coordination_id"], "OTV2-PREMIUM-DELIVERY");
    let policy = &manifest["policy"];
    assert_eq!(
        policy["max_authority_lease_seconds"],
        MAX_AUTHORITY_LEASE_US / 1_000_000
    );
    assert_eq!(
        policy["max_clock_skew_seconds"],
        MAX_CLOCK_SKEW_US / 1_000_000
    );
    assert_eq!(policy["max_response_bytes"], MAX_SNAPSHOT_BYTES);
    assert_eq!(policy["max_request_bytes"], MAX_REQUEST_BYTES);
    assert_eq!(policy["refresh_fraction"], "2/3");
    // No stale use: every Premium surface requires current authority.
    assert_eq!(policy["stale_within_bound"], "DENY");
    for surface in [
        Surface::FreshAdmission,
        Surface::ReconnectOrRecovery,
        Surface::RunningSession,
    ] {
        assert_eq!(surface_policy(surface), SurfacePolicy::RequireCurrent);
    }
    // The wire constants of the snapshot schema are Game's compatibility record.
    let schema = json("snapshot.schema.json");
    let properties = &schema["properties"];
    assert_eq!(properties["schema"]["const"], SNAPSHOT_SCHEMA);
    assert_eq!(properties["producer_profile"]["const"], PRODUCER_PROFILE);
    assert_eq!(properties["product_id"]["const"], PRODUCT_ID);
    assert_eq!(properties["product_version"]["const"], PRODUCT_VERSION);
    let request_schema = json("request.schema.json");
    assert_eq!(
        request_schema["properties"]["schema"]["const"],
        REQUEST_SCHEMA
    );
}

#[test]
fn games_request_is_the_fixture_request_on_the_contract_path() {
    assert_eq!(
        SNAPSHOT_PATH,
        "/internal/v1/products-entitlements/premium-snapshots/read"
    );
    let (account, nonce) = fixture_request();
    let body = request_body(account, &nonce).unwrap();
    assert!(body.len() <= MAX_REQUEST_BYTES);
    let ours: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(ours, json("request.valid.json"));
    assert_eq!(ours["account_id"], canonical_uuid(account));
    // Game never sends any of the requests Platform refuses.
    for case in manifest()["invalid_requests"].as_array().unwrap() {
        assert_ne!(ours, case["body"], "{}", case["case"]);
    }
}

#[test]
fn every_valid_fixture_is_accepted_as_its_state() {
    let (account, nonce) = fixture_request();
    let manifest = manifest();
    let listed: Vec<&str> = manifest["valid_snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["file"].as_str().unwrap())
        .collect();
    assert_eq!(listed, VALID, "the manifest's valid set is covered");
    for entry in manifest["valid_snapshots"].as_array().unwrap() {
        let file = entry["file"].as_str().unwrap();
        let evidence = validate(&bytes(file), account, &nonce)
            .unwrap_or_else(|failure| panic!("{file}: {failure:?}"));
        let state = match entry["entitlement_state"].as_str().unwrap() {
            "ACTIVE" => EntitlementState::Active,
            "NOT_YET_EFFECTIVE" => EntitlementState::NotYetEffective,
            "EXPIRED" => EntitlementState::Expired,
            "REVOKED" => EntitlementState::Revoked,
            "NONE" => EntitlementState::None,
            other => panic!("{file}: {other}"),
        };
        assert_eq!(evidence.state, state, "{file}");
        let wire = json(file);
        assert_eq!(
            evidence.authority_revision,
            wire["authority_revision"].as_u64().unwrap()
        );
        assert_eq!(
            evidence.entitlement_id.as_deref(),
            wire["entitlement_id"].as_str()
        );
    }
}

#[test]
fn every_invalid_fixture_is_refused_with_the_required_outcome() {
    let (account, nonce) = fixture_request();
    let cases = manifest()["invalid_snapshots"].as_array().unwrap().clone();
    let names: Vec<&str> = cases.iter().map(|c| c["case"].as_str().unwrap()).collect();
    let expected: Vec<&str> = INVALID.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, expected, "every Platform case has a Game outcome");
    for (case, (name, outcome)) in cases.iter().zip(INVALID) {
        let body = serde_json::to_vec(&case["body"]).unwrap();
        let rejection = validate(&body, account, &nonce)
            .map(|_| ())
            .map_err(|failure| failure.rejection());
        assert_eq!(rejection, Err(outcome), "{name}");
    }
}

#[test]
fn every_valid_fixture_reaches_validation_over_mutual_tls() {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (account, _) = fixture_request();
            let pki = TestPki::new();
            let client = PremiumSnapshotClient::new(&PremiumClientConfig {
                origin: String::new(),
                identity_pem: pki.client_identity_pem.clone().into_bytes(),
                platform_ca_pem: pki.ca_pem.clone().into_bytes(),
            });
            assert!(client.is_err(), "no origin, no client");
            for file in VALID {
                // The producer echoes the request nonce; everything else is the fixture.
                let fixture = json(file);
                let producer = TestProducer::start(&pki, move |request| {
                    let mut body = fixture.clone();
                    body["nonce"] = Value::String(request.nonce.clone());
                    Reply::json(serde_json::to_vec(&body).unwrap())
                })
                .await;
                let client = PremiumSnapshotClient::new(&PremiumClientConfig {
                    origin: producer.origin.clone(),
                    identity_pem: pki.client_identity_pem.clone().into_bytes(),
                    platform_ca_pem: pki.ca_pem.clone().into_bytes(),
                })
                .unwrap();
                let nonce = oteryn_game_server::premium::client::fresh_nonce().unwrap();
                let body = client.pull(account, &nonce).await.unwrap();
                let evidence = validate(&body, account, &nonce)
                    .unwrap_or_else(|failure| panic!("{file}: {failure:?}"));
                assert_eq!(
                    evidence.authority_revision,
                    json(file)["authority_revision"].as_u64().unwrap()
                );
                let requests = producer.requests();
                assert_eq!(requests.len(), 1);
                assert_eq!(requests[0].account_id, canonical_uuid(account));
                assert_eq!(requests[0].nonce, nonce);
            }
        });
}
