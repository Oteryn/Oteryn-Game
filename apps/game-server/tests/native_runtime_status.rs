//! `oteryn-game-native-runtime-status-v1` producer tests against a loopback
//! Platform stub. Every certificate is generated here for the test only.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use oteryn_game_server::native_admission_source::runtime_status::{
    self as rs, Delivery, Publication, RouteDescriptor, RuntimeStatusDescriptor,
};
use oteryn_game_server::native_admission_source::{SourceError, TransientCapacity};
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const PEER: &str = "platform.test";
const ACK: &str = r#"{"contract_version":1,"result":"accepted"}"#;

struct Identity {
    chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
}

struct Pki {
    ca: CertificateDer<'static>,
    server: Identity,
    status: Identity,
    evidence: Identity,
}

fn issue(
    ca: &CertifiedIssuer<'_, KeyPair>,
    name: &str,
    usage: ExtendedKeyUsagePurpose,
) -> Identity {
    let key = KeyPair::generate().unwrap();
    let mut params = CertificateParams::new(vec![name.to_owned()]).unwrap();
    params.extended_key_usages = vec![usage];
    let cert = params.signed_by(&key, ca).unwrap();
    Identity {
        chain: vec![cert.der().clone()],
        key: PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der())),
    }
}

fn pki() -> Pki {
    let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca = CertifiedIssuer::self_signed(params, KeyPair::generate().unwrap()).unwrap();
    Pki {
        ca: ca.der().clone(),
        server: issue(&ca, PEER, ExtendedKeyUsagePurpose::ServerAuth),
        status: issue(&ca, "node-a.status", ExtendedKeyUsagePurpose::ClientAuth),
        evidence: issue(&ca, "node-a.evidence", ExtendedKeyUsagePurpose::ClientAuth),
    }
}

type Seen = Arc<Mutex<Vec<(Instant, String, serde_json::Value)>>>;

/// Platform stub: accepts only the runtime-status identity (else `401`) and
/// records `(arrival, path, body)`.
async fn platform(pki: &Pki) -> (u16, Seen) {
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let mut roots = rustls::RootCertStore::empty();
    roots.add(pki.ca.clone()).unwrap();
    let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
        Arc::new(roots),
        provider.clone(),
    )
    .build()
    .unwrap();
    let mut config = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_client_cert_verifier(verifier)
        .with_single_cert(pki.server.chain.clone(), pki.server.key.clone_key())
        .unwrap();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen: Seen = Arc::default();
    let (allowed, record) = (pki.status.chain[0].clone(), seen.clone());
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let Ok(mut tls) = acceptor.accept(tcp).await else {
                continue;
            };
            let mut raw = Vec::new();
            let mut byte = [0u8];
            while !raw.ends_with(b"\r\n\r\n") && tls.read(&mut byte).await.unwrap_or(0) == 1 {
                raw.push(byte[0]);
            }
            let head = String::from_utf8(raw).unwrap();
            let path = head.split(' ').nth(1).unwrap_or_default().to_owned();
            let length: usize = head
                .lines()
                .find_map(|l| l.strip_prefix("Content-Length: "))
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let mut body = vec![0; length];
            let _ = tls.read_exact(&mut body).await;
            let identity = tls
                .get_ref()
                .1
                .peer_certificates()
                .and_then(|c| c.first().cloned());
            let response = if identity.as_ref() == Some(&allowed) {
                let value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
                record.lock().unwrap().push((Instant::now(), path, value));
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{ACK}",
                    ACK.len()
                )
            } else {
                "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n".to_owned()
            };
            let _ = tls.write_all(response.as_bytes()).await;
            let _ = tls.shutdown().await;
        }
    });
    (port, seen)
}

fn descriptor(
    pki: &Pki,
    port: u16,
    client: &Identity,
) -> Result<RuntimeStatusDescriptor, SourceError> {
    RuntimeStatusDescriptor::new(
        ("127.0.0.1".into(), port),
        PEER.into(),
        vec![pki.ca.clone()],
        client.chain.clone(),
        client.key.clone_key(),
        &[(&pki.evidence.chain, &pki.evidence.key)],
    )
}

fn publication(generation: u64, source_revision: u64, ready: bool) -> Publication {
    Publication {
        source_authority: "oteryn:runtime:world-1:channel-1".into(),
        world_id: "01934f10-7c02-7001-805b-3b1122334401".into(),
        channel_id: "01934f10-7c03-7001-805b-3b1122334401".into(),
        node_id: "01934f10-7c04-7001-805b-3b1122334401".into(),
        assignment_epoch: 1,
        scope_ownership_generation: generation,
        source_revision,
        decision_identity: format!("runtime-readiness:0a:{generation}:{source_revision}:{ready}"),
        ready,
        published_at: 1_790_000_000,
        protocol_major: 1,
        transport_profile: 1,
        route_revision: "rt.4.0f3a9c1d2b7e4a5f6c8d9e0a1b2c3d4e".into(),
        runtime_observation_revision: "observation-1".into(),
        ruleset_revision: "ruleset-1".into(),
        content_revision: "content-1".into(),
        map_revision: "map-1".into(),
        world_policy_revision: "policy-1".into(),
        offer_revision: "offer-1".into(),
    }
}

fn now() -> i64 {
    i64::try_from(std::time::UNIX_EPOCH.elapsed().unwrap().as_secs()).unwrap()
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}

#[test]
fn report_is_the_exact_contract_wire_without_any_endpoint() {
    let body = rs::encode(&publication(3, 7, true), 1_790_000_015).unwrap();
    assert_eq!(
        body,
        concat!(
            r#"{"contract_version":1,"operation":"ReportRuntimeStatusV1","#,
            r#""source_authority":"oteryn:runtime:world-1:channel-1","#,
            r#""world_id":"01934f10-7c02-7001-805b-3b1122334401","#,
            r#""channel_id":"01934f10-7c03-7001-805b-3b1122334401","#,
            r#""node_id":"01934f10-7c04-7001-805b-3b1122334401","#,
            r#""assignment_epoch":"1","scope_ownership_generation":"3","source_revision":"7","#,
            r#""decision_identity":"runtime-readiness:0a:3:7:true","ready":true,"#,
            r#""published_at":"1790000000","observed_at":"1790000015","#,
            r#""protocol_major":1,"transport_profile":1,"#,
            r#""route_revision":"rt.4.0f3a9c1d2b7e4a5f6c8d9e0a1b2c3d4e","#,
            r#""runtime_observation_revision":"observation-1","ruleset_revision":"ruleset-1","#,
            r#""content_revision":"content-1","map_revision":"map-1","#,
            r#""world_policy_revision":"policy-1","offer_revision":"offer-1"}"#
        )
    );
    let value: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&body).unwrap();
    assert_eq!(value.len(), 22);
    for key in [
        "host",
        "port",
        "endpoint",
        "tls_server_name",
        "alpn",
        "route",
    ] {
        assert!(!value.contains_key(key), "{key}");
    }
}

#[test]
fn encode_refuses_each_field_outside_the_grammar() {
    let mutations: [fn(&mut Publication, &mut i64); 10] = [
        |p, _| p.assignment_epoch = 0,
        |p, _| p.scope_ownership_generation = 0,
        |p, _| p.source_revision = 0,
        |_, observed| *observed = 1_789_999_999,
        |p, _| p.published_at = -1,
        |p, _| p.protocol_major = 2,
        |p, _| p.route_revision = "r".repeat(65),
        |p, _| p.offer_revision = "offer 1".into(),
        |p, _| p.world_id = p.world_id.to_uppercase(),
        |p, _| p.decision_identity = "a\"b".into(),
    ];
    for (index, mutate) in mutations.iter().enumerate() {
        let (mut p, mut observed) = (publication(3, 7, true), 1_790_000_015);
        mutate(&mut p, &mut observed);
        assert!(
            matches!(rs::encode(&p, observed), Err(SourceError::InvalidInput)),
            "{index}"
        );
    }
}

#[test]
fn response_is_exact() {
    for (raw, expected) in [
        (ACK, Delivery::Accepted),
        (
            r#"{"contract_version":1,"result":"refreshed"}"#,
            Delivery::Refreshed,
        ),
        (
            r#"{"contract_version":1,"result":"superseded"}"#,
            Delivery::Superseded,
        ),
    ] {
        assert_eq!(rs::decode_response(raw.as_bytes()).unwrap(), expected);
    }
    for raw in [
        r#"{"contract_version":2,"result":"accepted"}"#,
        r#"{"contract_version":1,"result":"accepted","x":1}"#,
        r#"{"contract_version":1,"result":"accepted","result":"accepted"}"#,
        r#"{"contract_version":1,"result":null}"#,
        r#"{"contract_version":1}"#,
        "",
    ] {
        assert!(rs::decode_response(raw.as_bytes()).is_err(), "{raw}");
    }
    // NRS-RESPONSE-BYTES: 256 decodes, 257 refuses before decoding.
    let at_cap = format!("{ACK}{}", " ".repeat(rs::RESPONSE_BYTES - ACK.len()));
    assert_eq!(
        rs::decode_response(at_cap.as_bytes()).unwrap(),
        Delivery::Accepted
    );
    let padded = format!("{at_cap} ");
    assert!(matches!(
        rs::decode_response(padded.as_bytes()),
        Err(SourceError::CapacityExceeded)
    ));
}

#[test]
fn the_largest_grammar_valid_report_fits_nrs_report_bytes() {
    let mut p = publication(u64::MAX, u64::MAX, false);
    p.assignment_epoch = u64::MAX;
    p.source_authority = "a".repeat(128);
    p.decision_identity = "d".repeat(128);
    p.published_at = i64::MAX;
    for revision in [
        &mut p.route_revision,
        &mut p.runtime_observation_revision,
        &mut p.ruleset_revision,
        &mut p.content_revision,
        &mut p.map_revision,
        &mut p.world_policy_revision,
        &mut p.offer_revision,
    ] {
        *revision = "r".repeat(64);
    }
    let body = rs::encode(&p, i64::MAX).unwrap();
    assert!(body.len() <= rs::REPORT_BYTES, "{}", body.len());
}

#[test]
fn route_revision_is_recomputed_only_for_comparison() {
    let mut route = RouteDescriptor {
        world_id: "01934f10-7c02-7001-805b-3b1122334401",
        channel_id: "01934f10-7c03-7001-805b-3b1122334401",
        host: "play.oteryn.test",
        port: 7172,
        tls_server_name: "play.oteryn.test",
        alpn: "oteryn-game/1",
        protocol_major: 1,
        transport_profile: 1,
    };
    // Independent vector: sorted-key, whitespace-free JSON, SHA-256, 32 hex.
    assert_eq!(
        rs::route_revision(4, &route),
        "rt.4.f90a1ac89742af0ccce28d2d0a95656a"
    );
    route.channel_id = "01934f10-7c03-7001-805b-3b1122334402";
    assert_ne!(
        rs::route_revision(4, &route),
        "rt.4.f90a1ac89742af0ccce28d2d0a95656a"
    );
}

#[test]
fn each_purpose_uses_its_own_certificate() {
    let pki = pki();
    // Reusing the evidence certificate or key for runtime status refuses locally.
    assert!(descriptor(&pki, 1, &pki.evidence).is_err());
    let mixed = Identity {
        chain: pki.status.chain.clone(),
        key: pki.evidence.key.clone_key(),
    };
    assert!(descriptor(&pki, 1, &mixed).is_err());
    block_on(async {
        let (port, seen) = platform(&pki).await;
        let capacity = TransientCapacity::new();
        let good = descriptor(&pki, port, &pki.status).unwrap();
        let p = publication(3, 7, true);
        assert_eq!(
            rs::deliver(&good, &capacity, &p, now()).await.unwrap(),
            Delivery::Accepted
        );
        // Platform refuses a cross-purpose identity (401): not delivered.
        let other = pki_identity_for_other_purpose(&pki);
        let cross = RuntimeStatusDescriptor::new(
            ("127.0.0.1".into(), port),
            PEER.into(),
            vec![pki.ca.clone()],
            other.chain,
            other.key,
            &[],
        )
        .unwrap();
        assert!(matches!(
            rs::deliver(&cross, &capacity, &p, now()).await,
            Err(SourceError::Unavailable)
        ));
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].1, "/internal/v1/game-auth/native-runtime-status");
    });
}

fn pki_identity_for_other_purpose(pki: &Pki) -> Identity {
    Identity {
        chain: pki.evidence.chain.clone(),
        key: pki.evidence.key.clone_key(),
    }
}

fn triple(value: &serde_json::Value) -> (u64, u64, u64, i64) {
    let field = |k: &str| value[k].as_str().unwrap().parse::<u64>().unwrap();
    (
        field("assignment_epoch"),
        field("scope_ownership_generation"),
        field("source_revision"),
        value["observed_at"].as_str().unwrap().parse().unwrap(),
    )
}

#[test]
fn heartbeat_cadence_ordering_and_stop_conditions() {
    let pki = pki();
    block_on(async {
        let (port, seen) = platform(&pki).await;
        let heartbeat = Duration::from_millis(200);
        let gate_open = Arc::new(AtomicBool::new(true));
        let gate = {
            let gate_open = gate_open.clone();
            move || std::future::ready(gate_open.load(Ordering::SeqCst))
        };
        let (sender, receiver) = tokio::sync::watch::channel(Some(publication(3, 1, true)));
        let task = tokio::spawn(rs::run(
            Arc::new(descriptor(&pki, port, &pki.status).unwrap()),
            receiver,
            heartbeat,
            now,
            gate,
        ));
        // Initial report plus heartbeats at H.
        tokio::time::sleep(Duration::from_millis(1_100)).await;
        let first = seen.lock().unwrap().len();
        assert!((4..=7).contains(&first), "{first}");
        // A newer publication supersedes; heartbeats continue with it.
        sender.send_replace(Some(publication(3, 2, true)));
        tokio::time::sleep(Duration::from_millis(500)).await;
        // Gate failure (durability root not ready, lost assignment, shutdown): heartbeats stop.
        gate_open.store(false, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(100)).await;
        let stopped = seen.lock().unwrap().len();
        tokio::time::sleep(Duration::from_millis(600)).await;
        assert_eq!(seen.lock().unwrap().len(), stopped);
        // The ready=false commit is reported once, never heartbeated.
        gate_open.store(true, Ordering::SeqCst);
        sender.send_replace(Some(publication(3, 3, false)));
        tokio::time::sleep(Duration::from_millis(700)).await;
        assert_eq!(seen.lock().unwrap().len(), stopped + 1);
        drop(sender);
        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap();

        let seen = seen.lock().unwrap();
        let mut previous = (0, 0, 0, 0);
        for window in seen.windows(2) {
            if window[0].2["source_revision"] == window[1].2["source_revision"] {
                assert!(window[1].0 - window[0].0 >= heartbeat - Duration::from_millis(20));
            }
        }
        for (_, _, value) in seen.iter() {
            let current = triple(value);
            assert!(current >= previous, "{current:?} < {previous:?}");
            previous = current;
            assert_eq!(value.as_object().unwrap().len(), 22);
        }
        assert_eq!(seen.last().unwrap().2["ready"], false);
    });
}

#[test]
fn platform_outage_never_blocks_the_node() {
    let pki = pki();
    block_on(async {
        // Accepts TCP and never answers.
        let hole = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = hole.local_addr().unwrap().port();
        let _hold = tokio::spawn(async move {
            let mut open = Vec::new();
            while let Ok((tcp, _)) = hole.accept().await {
                open.push(tcp);
            }
        });
        let started = Instant::now();
        let capacity = TransientCapacity::new();
        let target = descriptor(&pki, port, &pki.status).unwrap();
        assert!(matches!(
            rs::deliver(&target, &capacity, &publication(3, 1, true), now()).await,
            Err(SourceError::Unavailable)
        ));
        assert!(started.elapsed() < Duration::from_millis(3_500));

        let (sender, receiver) = tokio::sync::watch::channel(Some(publication(3, 1, true)));
        let task = tokio::spawn(rs::run(
            Arc::new(target),
            receiver,
            Duration::from_millis(100),
            now,
            || std::future::ready(true),
        ));
        tokio::time::sleep(Duration::from_millis(300)).await;
        // Publishing never waits on delivery, and the reporter ends within
        // one bounded exchange once the node stops.
        let publish = Instant::now();
        sender.send_replace(Some(publication(3, 2, false)));
        assert!(publish.elapsed() < Duration::from_millis(10));
        drop(sender);
        tokio::time::timeout(Duration::from_millis(6_500), task)
            .await
            .unwrap()
            .unwrap();
    });
}
