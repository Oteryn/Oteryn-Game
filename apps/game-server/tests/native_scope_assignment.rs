//! `ReportScopeAssignmentV1` and `ReportScopeRevocationV1`
//! (`oteryn-game-native-runtime-status-v1` §5, §16.1) producer tests against a
//! loopback Platform stub. Every certificate is
//! generated here for the test only.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use oteryn_game_server::native_admission_source::scope_assignment::{
    self as sa, Assignment, Delivery, NotDelivered, ReportConfig, RetryPolicy, Revocation,
    ScopeAssignmentDescriptor,
};
use oteryn_game_server::native_admission_source::{SourceError, TransientCapacity};
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, DistinguishedName, DnType,
    ExtendedKeyUsagePurpose, IsCa, KeyPair,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const PEER: &str = "platform.test";
const WORLD: &str = "01934f10-7c02-7001-805b-3b1122334401";
const CHANNEL: &str = "01934f10-7c03-7001-805b-3b1122334401";
const NODE: &str = "node-a.runtime-status";

struct Identity {
    chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
}

struct Pki {
    ca: CertificateDer<'static>,
    server: Identity,
    authority: Identity,
    status: Identity,
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
    let client = ExtendedKeyUsagePurpose::ClientAuth;
    Pki {
        ca: ca.der().clone(),
        server: issue(&ca, PEER, ExtendedKeyUsagePurpose::ServerAuth),
        authority: issue(&ca, "ops.scope-ownership-authority", client.clone()),
        status: issue(&ca, NODE, client),
    }
}

/// One scripted Platform answer: an optional stall before responding, then
/// the raw HTTP response.
struct Answer {
    stall: Duration,
    response: String,
}

fn answer(status: &str, body: &str) -> Answer {
    Answer {
        stall: Duration::ZERO,
        response: format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        ),
    }
}

fn result(result: &str) -> Answer {
    answer(
        "200 OK",
        &format!(r#"{{"contract_version":1,"result":"{result}"}}"#),
    )
}

type Seen = Arc<Mutex<Vec<(String, Vec<u8>)>>>;
/// Answers one authenticated `(path, raw body)`; `None` answers `401`.
type Handler = Arc<dyn Fn(&str, &[u8]) -> Option<Answer> + Send + Sync>;

/// Platform stub: each authenticated request takes the next scripted answer.
async fn platform(pki: &Pki, script: Vec<Answer>) -> (u16, Seen) {
    let script = Mutex::new(VecDeque::from(script));
    serve(
        pki,
        Arc::new(move |_, _| script.lock().unwrap().pop_front()),
    )
    .await
}

/// Platform stub: only the ownership-authority identity may report (else
/// `401`). Each authenticated request is recorded as `(path, raw body)` and
/// answered by `handler`.
async fn serve(pki: &Pki, handler: Handler) -> (u16, Seen) {
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
    let (allowed, record) = (pki.authority.chain[0].clone(), seen.clone());
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let Ok(mut tls) = acceptor.accept(tcp).await else {
                continue;
            };
            let (allowed, record, handler) = (allowed.clone(), record.clone(), handler.clone());
            tokio::spawn(async move {
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
                let next = if identity.as_ref() == Some(&allowed) {
                    let next = handler(&path, &body);
                    record.lock().unwrap().push((path, body));
                    next
                } else {
                    None
                };
                let next = next.unwrap_or_else(|| answer("401 Unauthorized", ""));
                tokio::time::sleep(next.stall).await;
                let _ = tls.write_all(next.response.as_bytes()).await;
                let _ = tls.shutdown().await;
            });
        }
    });
    (port, seen)
}

fn descriptor(
    pki: &Pki,
    port: u16,
    client: &Identity,
) -> Result<ScopeAssignmentDescriptor, SourceError> {
    ScopeAssignmentDescriptor::new(
        ("127.0.0.1".into(), port),
        PEER.into(),
        vec![pki.ca.clone()],
        client.chain.clone(),
        client.key.clone_key(),
        &[],
    )
}

fn assignment(generation: u64) -> Assignment {
    Assignment {
        assignment_epoch: 1,
        world_id: WORLD.into(),
        channel_id: CHANNEL.into(),
        ownership_generation: generation,
        node_identity: NODE.into(),
        assigned_at: 1_789_999_990,
    }
}

fn fast() -> RetryPolicy {
    RetryPolicy {
        first: Duration::from_millis(10),
        max: Duration::from_millis(40),
        attempts: 4,
    }
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}

#[test]
fn report_is_the_exact_contract_wire() {
    assert_eq!(
        sa::encode(&assignment(3)).unwrap(),
        concat!(
            r#"{"contract_version":1,"operation":"ReportScopeAssignmentV1","#,
            r#""assignment_epoch":"1","#,
            r#""world_id":"01934f10-7c02-7001-805b-3b1122334401","#,
            r#""channel_id":"01934f10-7c03-7001-805b-3b1122334401","#,
            r#""ownership_generation":"3","node_identity":"node-a.runtime-status","#,
            r#""assigned_at":"1789999990"}"#
        )
    );
}

#[test]
fn report_outside_the_grammar_is_refused_before_sending() {
    let cases: [fn(&mut Assignment); 7] = [
        |a| a.assignment_epoch = 0,
        |a| a.ownership_generation = 0,
        |a| a.world_id = "01934F10-7C02-7001-805B-3B1122334401".into(),
        |a| a.channel_id = "01934f10-7c03-4001-805b-3b1122334401".into(),
        |a| a.channel_id = WORLD.into(),
        |a| a.node_identity = "node\"a".into(),
        |a| a.assigned_at = -1,
    ];
    for mutate in cases {
        let mut a = assignment(3);
        mutate(&mut a);
        assert!(matches!(sa::encode(&a), Err(SourceError::InvalidInput)));
    }
    let mut long = assignment(3);
    long.node_identity = "n".repeat(sa::NODE_IDENTITY_BYTES + 1);
    assert!(sa::encode(&long).is_err());
}

#[test]
fn response_decoder_is_exact() {
    assert_eq!(
        sa::decode_response(br#"{"contract_version":1,"result":"accepted"}"#).unwrap(),
        Delivery::Accepted
    );
    assert_eq!(
        sa::decode_response(br#"{"contract_version":1,"result":"superseded"}"#).unwrap(),
        Delivery::Superseded
    );
    for raw in [
        &br#"{"contract_version":1,"result":"refreshed"}"#[..],
        br#"{"contract_version":2,"result":"accepted"}"#,
        br#"{"contract_version":1,"result":"accepted","extra":1}"#,
        br#"{"contract_version":1,"result":null}"#,
        br#"{"result":"accepted"}"#,
        b"",
    ] {
        assert!(sa::decode_response(raw).is_err());
    }
}

#[test]
fn accepted_report_goes_to_the_compiled_path() {
    block_on(async {
        let pki = pki();
        let (port, seen) = platform(&pki, vec![result("accepted")]).await;
        let d = descriptor(&pki, port, &pki.authority).unwrap();
        let report = sa::report(&d, &assignment(3), fast()).await;
        assert_eq!(report.result, Ok(Delivery::Accepted));
        assert_eq!(report.attempts, 1);
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, sa::PATH);
        assert_eq!(seen[0].1, sa::encode(&assignment(3)).unwrap().as_bytes());
    });
}

#[test]
fn superseded_report_is_a_definite_success() {
    block_on(async {
        let pki = pki();
        let (port, seen) = platform(&pki, vec![result("superseded")]).await;
        let d = descriptor(&pki, port, &pki.authority).unwrap();
        let report = sa::report(&d, &assignment(2), fast()).await;
        assert_eq!(report.result, Ok(Delivery::Superseded));
        assert_eq!(report.attempts, 1);
        assert_eq!(seen.lock().unwrap().len(), 1);
    });
}

#[test]
fn definite_refusals_stop_without_retry() {
    for (status, class) in [
        ("400 Bad Request", NotDelivered::Malformed),
        ("401 Unauthorized", NotDelivered::Unauthenticated),
        ("409 Conflict", NotDelivered::Conflict),
    ] {
        block_on(async {
            let pki = pki();
            let (port, seen) = platform(&pki, vec![answer(status, ""), result("accepted")]).await;
            let d = descriptor(&pki, port, &pki.authority).unwrap();
            let report = sa::report(&d, &assignment(3), fast()).await;
            assert_eq!(report.result, Err(class));
            assert!(class.definite());
            assert_eq!(report.attempts, 1);
            assert_eq!(seen.lock().unwrap().len(), 1);
        });
    }
}

#[test]
fn wrong_purpose_identity_is_unauthenticated() {
    block_on(async {
        let pki = pki();
        let (port, seen) = platform(&pki, vec![result("accepted")]).await;
        let d = descriptor(&pki, port, &pki.status).unwrap();
        let report = sa::report(&d, &assignment(3), fast()).await;
        assert_eq!(report.result, Err(NotDelivered::Unauthenticated));
        assert_eq!(report.attempts, 1);
        assert!(seen.lock().unwrap().is_empty());
    });
}

#[test]
fn transient_failures_retry_with_identical_bytes_until_accepted() {
    block_on(async {
        let pki = pki();
        let timeout = Answer {
            stall: Duration::from_millis(3_500),
            ..result("accepted")
        };
        let script = vec![
            timeout,
            answer("503 Service Unavailable", ""),
            answer("429 Too Many Requests", ""),
            result("accepted"),
        ];
        let (port, seen) = platform(&pki, script).await;
        let d = descriptor(&pki, port, &pki.authority).unwrap();
        let started = Instant::now();
        let report = sa::report(&d, &assignment(3), fast()).await;
        assert_eq!(report.result, Ok(Delivery::Accepted));
        assert_eq!(report.attempts, 4);
        assert!(started.elapsed() >= Duration::from_secs(3));
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 4);
        assert!(
            seen.iter()
                .all(|(path, body)| path == sa::PATH && *body == seen[0].1)
        );
    });
}

#[test]
fn exhausted_retries_return_the_last_indefinite_outcome() {
    block_on(async {
        let pki = pki();
        let script = (0..4)
            .map(|_| answer("503 Service Unavailable", ""))
            .collect();
        let (port, seen) = platform(&pki, script).await;
        let d = descriptor(&pki, port, &pki.authority).unwrap();
        let report = sa::report(&d, &assignment(3), fast()).await;
        assert_eq!(report.result, Err(NotDelivered::Unavailable));
        assert!(!NotDelivered::Unavailable.definite());
        assert_eq!(report.attempts, 4);
        assert_eq!(seen.lock().unwrap().len(), 4);
    });
}

#[test]
fn refusal_with_a_body_is_an_invalid_response() {
    block_on(async {
        let pki = pki();
        let (port, _) = platform(&pki, vec![answer("409 Conflict", "no")]).await;
        let d = descriptor(&pki, port, &pki.authority).unwrap();
        let capacity = TransientCapacity::new();
        let body = sa::encode(&assignment(3)).unwrap();
        assert_eq!(
            sa::deliver(&d, &capacity, &body).await,
            Err(NotDelivered::InvalidResponse)
        );
    });
}

#[test]
fn response_over_the_bound_is_refused_in_transport_for_every_status() {
    block_on(async {
        let pki = pki();
        let oversized = " ".repeat(sa::RESPONSE_BYTES + 1);
        let half = " ".repeat(sa::RESPONSE_BYTES / 2 + 1);
        let raw = |response: String| Answer {
            stall: Duration::ZERO,
            response,
        };
        let script = vec![
            answer("200 OK", &oversized),
            answer("409 Conflict", &oversized),
            // Chunked: no single chunk exceeds the bound, their sum does.
            raw(format!(
                "HTTP/1.1 503 Service Unavailable\r\nTransfer-Encoding: chunked\r\n\r\n{0:x}\r\n{1}\r\n{0:x}\r\n{1}\r\n0\r\n\r\n",
                half.len(),
                half
            )),
            // Close-delimited: no length is declared.
            raw(format!("HTTP/1.1 400 Bad Request\r\n\r\n{oversized}")),
            answer("200 OK", &" ".repeat(sa::RESPONSE_BYTES)),
        ];
        let (port, _) = platform(&pki, script).await;
        let d = descriptor(&pki, port, &pki.authority).unwrap();
        let capacity = TransientCapacity::new();
        let body = sa::encode(&assignment(3)).unwrap();
        for _ in 0..4 {
            assert_eq!(
                sa::deliver(&d, &capacity, &body).await,
                Err(NotDelivered::Unavailable)
            );
        }
        // At the bound the body reaches the exact decoder.
        assert_eq!(
            sa::deliver(&d, &capacity, &body).await,
            Err(NotDelivered::InvalidResponse)
        );
    });
}

#[test]
fn re_sending_the_same_assignment_sends_identical_bytes() {
    block_on(async {
        let pki = pki();
        let (port, seen) = platform(&pki, vec![result("accepted"), result("accepted")]).await;
        let d = descriptor(&pki, port, &pki.authority).unwrap();
        for _ in 0..2 {
            let report = sa::report(&d, &assignment(3), fast()).await;
            assert_eq!(report.result, Ok(Delivery::Accepted));
        }
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].1, seen[1].1);
    });
}

#[test]
fn descriptor_refuses_a_key_of_another_leaf_and_a_shared_leaf() {
    let pki = pki();
    assert!(
        ScopeAssignmentDescriptor::new(
            ("127.0.0.1".into(), 1),
            PEER.into(),
            vec![pki.ca.clone()],
            pki.authority.chain.clone(),
            pki.status.key.clone_key(),
            &[],
        )
        .is_err()
    );
    assert!(
        ScopeAssignmentDescriptor::new(
            ("127.0.0.1".into(), 1),
            PEER.into(),
            vec![pki.ca.clone()],
            pki.authority.chain.clone(),
            pki.authority.key.clone_key(),
            &[&pki.authority.chain],
        )
        .is_err()
    );
}

const CONFIG: &str = r#"
endpoint = "127.0.0.1:8443"
peer_name = "platform.test"
trust_roots_file = "/etc/oteryn/platform-roots.pem"
client_certificate_file = "/etc/oteryn/scope-authority.pem"
client_key_file = "/etc/oteryn/scope-authority.key"
other_producer_certificate_files = ["/etc/oteryn/node/platform-client.crt", "/etc/oteryn/node/runtime-status.crt"]
node_certificate_files = { "node-a.runtime-status" = "/etc/oteryn/nodes/node-a.crt", "node-b.runtime-status" = "/etc/oteryn/nodes/node-b.crt" }
assignment_epoch = 1

[[scope]]
world_id = "01934f10-7c02-7001-805b-3b1122334401"
channel_id = "01934f10-7c03-7001-805b-3b1122334401"
node_identities = ["node-a.runtime-status", "node-b.runtime-status"]
"#;

#[test]
fn report_config_binds_node_identities_to_scopes() {
    let config = ReportConfig::parse(CONFIG.as_bytes()).unwrap();
    assert_eq!(config.assignment_epoch, 1);
    assert!(config.allows(WORLD, CHANNEL, NODE));
    assert!(config.allows(WORLD, CHANNEL, "node-b.runtime-status"));
    assert!(!config.allows(WORLD, CHANNEL, "node-c.runtime-status"));
    assert!(!config.allows(CHANNEL, WORLD, NODE));
}

#[test]
fn report_config_channel_must_match_the_node_platform_channel() {
    let config = ReportConfig::parse(CONFIG.as_bytes()).unwrap();
    let endpoint = "127.0.0.1:8443".parse().unwrap();
    assert!(config.matches_node_channel(endpoint, "platform.test", 1));
    assert!(!config.matches_node_channel(endpoint, "platform.test", 2));
    assert!(!config.matches_node_channel(endpoint, "platform.other", 1));
    assert!(!config.matches_node_channel("127.0.0.1:8444".parse().unwrap(), "platform.test", 1));
}

#[test]
fn report_config_requires_a_certificate_for_every_node_identity() {
    let config = ReportConfig::parse(CONFIG.as_bytes()).unwrap();
    assert_eq!(config.node_certificate_files.len(), 2);
    let node_b = r#", "node-b.runtime-status" = "/etc/oteryn/nodes/node-b.crt""#;
    for (from, to) in [
        // An omitted node host.
        (node_b, ""),
        // A certificate for an identity no scope names.
        (
            node_b,
            r#", "node-b.runtime-status" = "/etc/oteryn/nodes/node-b.crt", "node-c.runtime-status" = "/etc/oteryn/nodes/node-c.crt""#,
        ),
        // Shared with another producer path or the authority certificate.
        (
            "/etc/oteryn/nodes/node-b.crt",
            "/etc/oteryn/node/runtime-status.crt",
        ),
        (
            "/etc/oteryn/nodes/node-b.crt",
            "/etc/oteryn/nodes/node-a.crt",
        ),
        (
            "/etc/oteryn/nodes/node-b.crt",
            "/etc/oteryn/scope-authority.pem",
        ),
        ("/etc/oteryn/nodes/node-b.crt", "node-b.crt"),
    ] {
        let document = CONFIG.replace(from, to);
        assert_ne!(document, CONFIG);
        assert!(ReportConfig::parse(document.as_bytes()).is_err(), "{to}");
    }
    let without = CONFIG
        .lines()
        .filter(|line| !line.starts_with("node_certificate_files"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(ReportConfig::parse(without.as_bytes()).is_err());
}

#[test]
fn report_config_refuses_invalid_documents() {
    for (from, to) in [
        ("assignment_epoch = 1", "assignment_epoch = 0"),
        (
            "assignment_epoch = 1",
            "assignment_epoch = 1\npath = \"/x\"",
        ),
        ("/etc/oteryn/scope-authority.key", "scope-authority.key"),
        (
            r#"["node-a.runtime-status", "node-b.runtime-status"]"#,
            r#"["node-a.runtime-status", "node-a.runtime-status"]"#,
        ),
        (
            r#"["node-a.runtime-status", "node-b.runtime-status"]"#,
            "[]",
        ),
        ("7c03-7001", "7c03-4001"),
        ("assignment_epoch = 1\n", ""),
        (
            r#"["/etc/oteryn/node/platform-client.crt", "/etc/oteryn/node/runtime-status.crt"]"#,
            r#"["/etc/oteryn/node/platform-client.crt", "/etc/oteryn/node/platform-client.crt"]"#,
        ),
        (
            "/etc/oteryn/node/runtime-status.crt",
            "/etc/oteryn/scope-authority.pem",
        ),
        ("/etc/oteryn/node/runtime-status.crt", "runtime-status.crt"),
    ] {
        let document = CONFIG.replace(from, to);
        assert_ne!(document, CONFIG);
        assert!(ReportConfig::parse(document.as_bytes()).is_err(), "{from}");
    }
    let duplicate = format!(
        "{CONFIG}\n[[scope]]\nworld_id = \"{WORLD}\"\nchannel_id = \"{CHANNEL}\"\nnode_identities = [\"{NODE}\"]\n"
    );
    assert!(ReportConfig::parse(duplicate.as_bytes()).is_err());
}

fn subject_certificate(subject: &[(DnType, &str)]) -> CertificateDer<'static> {
    let mut params = CertificateParams::new(vec![NODE.to_owned()]).unwrap();
    let mut name = DistinguishedName::new();
    for (kind, value) in subject {
        name.push(kind.clone(), *value);
    }
    params.distinguished_name = name;
    params
        .self_signed(&KeyPair::generate().unwrap())
        .unwrap()
        .der()
        .clone()
}

#[test]
fn node_certificate_must_carry_its_configured_node_identity() {
    let node_a = subject_certificate(&[(DnType::CommonName, NODE)]);
    assert!(sa::certificate_has_node_identity(
        &node_a,
        "CN=node-a.runtime-status"
    ));
    // The bare common name is not the complete subject.
    assert!(!sa::certificate_has_node_identity(&node_a, NODE));
    // Node B's certificate configured under node A's key is refused.
    let node_b = subject_certificate(&[(DnType::CommonName, "node-b.runtime-status")]);
    assert!(!sa::certificate_has_node_identity(
        &node_b,
        "CN=node-a.runtime-status"
    ));
    assert!(sa::certificate_has_node_identity(
        &node_b,
        "CN=node-b.runtime-status"
    ));
    // Extra attributes are part of the subject.
    let full = subject_certificate(&[
        (DnType::OrganizationName, "Oteryn"),
        (DnType::CommonName, NODE),
    ]);
    assert!(!sa::certificate_has_node_identity(&full, NODE));
    assert!(!sa::certificate_has_node_identity(
        &full,
        "CN=node-a.runtime-status"
    ));
    assert!(sa::certificate_has_node_identity(
        &full,
        "CN=node-a.runtime-status,O=Oteryn"
    ));
    assert!(!sa::certificate_has_node_identity(
        &full,
        "O=Oteryn,CN=node-a.runtime-status"
    ));
    // A subject alternative name alone is not the subject.
    let no_subject = subject_certificate(&[(DnType::OrganizationName, "Oteryn")]);
    assert!(!sa::certificate_has_node_identity(&no_subject, NODE));
    // A value that needs escaping never matches.
    let escaped = subject_certificate(&[(DnType::CommonName, "node-a,O=x")]);
    assert!(!sa::certificate_has_node_identity(
        &escaped,
        "CN=node-a,O=x"
    ));
    assert!(!sa::certificate_has_node_identity(
        &CertificateDer::from(vec![0x30, 0x03, 0x02, 0x01, 0x01]),
        NODE
    ));
}

// ---------------------------------------------------------------------------
// `ReportScopeRevocationV1` (§16.1)
// ---------------------------------------------------------------------------

/// The contract §16.1 fixture, compact.
const REVOCATION_FIXTURE: &str = concat!(
    r#"{"contract_version":1,"operation":"ReportScopeRevocationV1","#,
    r#""assignment_epoch":"1","#,
    r#""world_id":"01934f10-7c02-7001-805b-3b1122334401","#,
    r#""channel_id":"01934f10-7c03-7001-805b-3b1122334401","#,
    r#""ownership_generation":"4","revoked_at":"1790000100"}"#
);

const REVOCATION_MEMBERS: [&str; 7] = [
    "contract_version",
    "operation",
    "assignment_epoch",
    "world_id",
    "channel_id",
    "ownership_generation",
    "revoked_at",
];

fn revocation(generation: u64) -> Revocation {
    Revocation {
        assignment_epoch: 1,
        world_id: WORLD.into(),
        channel_id: CHANNEL.into(),
        ownership_generation: generation,
        revoked_at: 1_790_000_100,
    }
}

/// The valid fixture with one member's raw JSON value replaced.
fn with_value(member: &str, raw: &str) -> String {
    let start = REVOCATION_FIXTURE.find(&format!(r#""{member}":"#)).unwrap() + member.len() + 3;
    let end = start + REVOCATION_FIXTURE[start..].find([',', '}']).unwrap();
    format!(
        "{}{raw}{}",
        &REVOCATION_FIXTURE[..start],
        &REVOCATION_FIXTURE[end..]
    )
}

/// Contract §13 negative and value fixtures for `ReportScopeRevocationV1`,
/// shared with the Platform consumer, which refuses each with `400`.
fn revocation_refusal_fixtures() -> Vec<(String, String)> {
    let mut fixtures: Vec<(String, String)> = vec![
        (
            "unknown member".into(),
            REVOCATION_FIXTURE.replace(
                r#""revoked_at""#,
                r#""node_identity":"node-a.runtime-status","revoked_at""#,
            ),
        ),
        (
            "duplicate member".into(),
            REVOCATION_FIXTURE.replace(
                r#""revoked_at""#,
                r#""ownership_generation":"4","revoked_at""#,
            ),
        ),
        ("null member".into(), with_value("revoked_at", "null")),
        (
            "missing member".into(),
            REVOCATION_FIXTURE.replace(r#","revoked_at":"1790000100""#, ""),
        ),
        (
            "nested object".into(),
            with_value(
                "world_id",
                r#"{"id":"01934f10-7c02-7001-805b-3b1122334401"}"#,
            ),
        ),
        (
            "over-long body".into(),
            REVOCATION_FIXTURE.replace(
                r#""revoked_at""#,
                &format!(r#"{}"revoked_at""#, " ".repeat(sa::REPORT_BYTES)),
            ),
        ),
    ];
    let values: &[(&str, &[&str])] = &[
        (
            "operation",
            &[
                r#""ReportScopeAssignmentV1""#,
                r#""ReportRuntimeStatusV1""#,
                r#""reportscoperevocationv1""#,
                r#""REPORTSCOPEREVOCATIONV1""#,
                r#""ReportScopeRevocationV2""#,
                "1",
            ],
        ),
        (
            "contract_version",
            &[r#""1""#, "2", "0", "1.0", "1e0", "-1", "true"],
        ),
        (
            "world_id",
            &[
                r#""01934F10-7C02-7001-805B-3B1122334401""#,
                r#""{01934f10-7c02-7001-805b-3b1122334401}""#,
                r#""01934f107c027001805b3b1122334401""#,
                r#""01934f10-7c02-4001-805b-3b1122334401""#,
                r#""01934f10-7c02-7001-805b-3b112233440""#,
                r#""01934f10-7c02-7001-805b-3b112233440100""#,
                "1934",
            ],
        ),
        (
            "channel_id",
            &[
                r#""01934F10-7C03-7001-805B-3B1122334401""#,
                r#""{01934f10-7c03-7001-805b-3b1122334401}""#,
                r#""01934f107c037001805b3b1122334401""#,
                r#""01934f10-7c03-4001-805b-3b1122334401""#,
                r#""01934f10-7c03-7001-805b-3b112233440""#,
                r#""01934f10-7c03-7001-805b-3b112233440100""#,
                "1934",
            ],
        ),
    ];
    let counter = [
        r#""0""#,
        r#""04""#,
        r#""+4""#,
        r#""-4""#,
        r#""4.0""#,
        r#""4e0""#,
        r#"" 4""#,
        r#""4 ""#,
        r#""18446744073709551616""#,
        "4",
    ];
    let time = [
        r#""-1""#,
        r#""01790000100""#,
        r#""+1790000100""#,
        r#""1790000100.0""#,
        r#""1.79e9""#,
        r#"" 1790000100""#,
        r#""18446744073709551616""#,
        "1790000100",
    ];
    let mut push = |member: &str, raws: &[&str]| {
        for raw in raws {
            fixtures.push((format!("{member}={raw}"), with_value(member, raw)));
        }
    };
    for (member, raws) in values {
        push(member, raws);
    }
    push("assignment_epoch", &counter);
    push("ownership_generation", &counter);
    push("revoked_at", &time);
    fixtures
}

/// A deliberately lenient reading of a fixture as encoder input: numbers and
/// strings are both accepted and parsed loosely, so that as many fixtures as
/// possible reach the encoder.
fn lenient_revocation(body: &str) -> Option<Revocation> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let object = value.as_object()?;
    let text = |member: &str| match object.get(member)? {
        serde_json::Value::String(text) => Some(text.trim().to_owned()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        _ => None,
    };
    Some(Revocation {
        assignment_epoch: text("assignment_epoch")?
            .trim_start_matches('+')
            .parse()
            .ok()?,
        world_id: text("world_id")?,
        channel_id: text("channel_id")?,
        ownership_generation: text("ownership_generation")?
            .trim_start_matches('+')
            .parse()
            .ok()?,
        revoked_at: text("revoked_at")?.trim_start_matches('+').parse().ok()?,
    })
}

#[test]
fn revocation_is_the_exact_contract_wire() {
    let body = sa::encode_revocation(&revocation(4)).unwrap();
    assert_eq!(body, REVOCATION_FIXTURE);
    // The exact member set: no node identity, nothing nested.
    let value: serde_json::Value = serde_json::from_str(&body).unwrap();
    let object = value.as_object().unwrap();
    let mut members: Vec<&str> = object.keys().map(String::as_str).collect();
    members.sort_unstable();
    let mut expected = REVOCATION_MEMBERS.to_vec();
    expected.sort_unstable();
    assert_eq!(members, expected);
    assert!(object.values().all(|v| v.is_string() || v.is_number()));
    // The valid fixture read back re-encodes to itself.
    assert_eq!(
        sa::encode_revocation(&lenient_revocation(REVOCATION_FIXTURE).unwrap()).unwrap(),
        REVOCATION_FIXTURE
    );
}

#[test]
fn revocation_is_byte_identical_after_a_restart() {
    // Only the durable row and the declared epoch feed the body: a fresh
    // process deriving it again produces the same bytes.
    let first = sa::encode_revocation(&revocation(4)).unwrap();
    let restarted = sa::encode_revocation(&Revocation {
        assignment_epoch: 1,
        world_id: String::from(WORLD),
        channel_id: String::from(CHANNEL),
        ownership_generation: 4,
        revoked_at: 1_790_000_100,
    })
    .unwrap();
    assert_eq!(first, restarted);
}

#[test]
fn revocation_refusal_fixtures_are_refused_by_construction() {
    let fixtures = revocation_refusal_fixtures();
    assert!(fixtures.len() > 50);
    for (name, body) in &fixtures {
        assert_ne!(body, REVOCATION_FIXTURE, "{name}");
        if name == "over-long body" {
            assert!(body.len() > sa::REPORT_BYTES);
        }
        // Whatever a lenient reader makes of the fixture, the encoder either
        // refuses it or emits other bytes: it cannot emit the fixture.
        if let Some(input) = lenient_revocation(body)
            && let Ok(encoded) = sa::encode_revocation(&input)
        {
            assert_ne!(&encoded, body, "{name}");
        }
    }
    // The encoder's output for every grammar-valid input is canonical, so no
    // fixture is among the bodies the encoder emits.
    for generation in [1, 4, 10, u64::MAX] {
        for epoch in [1, 4, u64::MAX] {
            let mut r = revocation(generation);
            r.assignment_epoch = epoch;
            let encoded = sa::encode_revocation(&r).unwrap();
            assert!(fixtures.iter().all(|(_, body)| body != &encoded));
        }
    }
}

#[test]
fn revocation_outside_the_grammar_is_refused_before_sending() {
    let cases: [fn(&mut Revocation); 7] = [
        |r| r.assignment_epoch = 0,
        |r| r.ownership_generation = 0,
        |r| r.world_id = "01934F10-7C02-7001-805B-3B1122334401".into(),
        |r| r.world_id = "{01934f10-7c02-7001-805b-3b1122334401}".into(),
        |r| r.channel_id = "01934f10-7c03-4001-805b-3b1122334401".into(),
        |r| r.channel_id = WORLD.into(),
        |r| r.revoked_at = -1,
    ];
    for mutate in cases {
        let mut r = revocation(4);
        mutate(&mut r);
        assert!(matches!(
            sa::encode_revocation(&r),
            Err(SourceError::InvalidInput)
        ));
    }
}

#[test]
fn revocation_response_decoder_accepts_only_the_exact_success_bodies() {
    assert_eq!(
        sa::decode_revocation_response(br#"{"contract_version":1,"result":"accepted"}"#).unwrap(),
        Delivery::Accepted
    );
    assert_eq!(
        sa::decode_revocation_response(br#"{"contract_version":1,"result":"superseded"}"#).unwrap(),
        Delivery::Superseded
    );
}

/// One test per contract §13 response shape: each is "not delivered".
macro_rules! revocation_response_refused {
    ($($name:ident: [$($raw:expr),+ $(,)?];)+) => {$(
        #[test]
        fn $name() {
            for raw in [$(&$raw[..]),+] {
                assert!(
                    sa::decode_revocation_response(raw).is_err(),
                    "{}",
                    String::from_utf8_lossy(raw)
                );
            }
        }
    )+};
}

revocation_response_refused! {
    revocation_response_with_an_unknown_member_is_not_delivered: [
        br#"{"contract_version":1,"result":"accepted","extra":1}"#,
        br#"{"contract_version":1,"result":"accepted","node_identity":"node-a"}"#,
    ];
    revocation_response_with_a_duplicate_member_is_not_delivered: [
        br#"{"contract_version":1,"result":"accepted","result":"accepted"}"#,
        br#"{"contract_version":1,"contract_version":1,"result":"accepted"}"#,
    ];
    revocation_response_with_a_null_member_is_not_delivered: [
        br#"{"contract_version":1,"result":null}"#,
        br#"{"contract_version":null,"result":"accepted"}"#,
    ];
    revocation_response_without_contract_version_is_not_delivered: [
        br#"{"result":"accepted"}"#,
    ];
    revocation_response_without_result_is_not_delivered: [
        br#"{"contract_version":1}"#,
    ];
    revocation_response_with_a_wrong_contract_version_is_not_delivered: [
        br#"{"contract_version":"1","result":"accepted"}"#,
        br#"{"contract_version":2,"result":"accepted"}"#,
        br#"{"contract_version":1.0,"result":"accepted"}"#,
        br#"{"contract_version":0,"result":"accepted"}"#,
    ];
    revocation_response_with_a_wrong_result_is_not_delivered: [
        br#"{"contract_version":1,"result":"refreshed"}"#,
        br#"{"contract_version":1,"result":"Accepted"}"#,
        br#"{"contract_version":1,"result":"rejected"}"#,
        br#"{"contract_version":1,"result":1}"#,
        br#"{"contract_version":1,"result":["accepted"]}"#,
        br#"{"contract_version":1,"result":true}"#,
    ];
    revocation_response_that_is_not_an_object_is_not_delivered: [
        b"",
        b"null",
        br#""accepted""#,
        b"1",
        br#"[{"contract_version":1,"result":"accepted"}]"#,
    ];
    revocation_response_with_trailing_bytes_is_not_delivered: [
        b"{\"contract_version\":1,\"result\":\"accepted\"}\n",
        br#"{"contract_version":1,"result":"accepted"} "#,
        br#"{"contract_version":1,"result":"accepted"}{}"#,
    ];
}

#[derive(Default)]
struct Ledger {
    /// Latest entry per scope: `(epoch, generation, raw body)`.
    latest: BTreeMap<(String, String), (u64, u64, Vec<u8>)>,
    /// Number of state changes.
    changes: usize,
}

/// A Platform consumer ordering assignment and revocation reports of one
/// scope as a single sequence by `(assignment_epoch, ownership_generation)`
/// (§16.1). Without `revocations` it has no revocation endpoint (`404`).
fn ordering_platform(revocations: bool) -> (Handler, Arc<Mutex<Ledger>>) {
    let ledger = Arc::new(Mutex::new(Ledger::default()));
    let state = ledger.clone();
    let handler: Handler = Arc::new(move |path, body| {
        if path != sa::PATH && !(revocations && path == sa::REVOCATION_PATH) {
            return Some(answer("404 Not Found", ""));
        }
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) else {
            return Some(answer("400 Bad Request", ""));
        };
        let member = |name: &str| value[name].as_str().map(str::to_owned);
        let counter = |name: &str| member(name).and_then(|v| v.parse::<u64>().ok());
        let (Some(world), Some(channel), Some(epoch), Some(generation)) = (
            member("world_id"),
            member("channel_id"),
            counter("assignment_epoch"),
            counter("ownership_generation"),
        ) else {
            return Some(answer("400 Bad Request", ""));
        };
        let mut ledger = state.lock().unwrap();
        let next = match ledger.latest.get(&(world.clone(), channel.clone())) {
            Some((e, g, _)) if (*e, *g) > (epoch, generation) => result("superseded"),
            Some((e, g, latest)) if (*e, *g) == (epoch, generation) => {
                if latest == body {
                    result("accepted")
                } else {
                    answer("409 Conflict", "")
                }
            }
            _ => {
                ledger
                    .latest
                    .insert((world, channel), (epoch, generation, body.to_vec()));
                ledger.changes += 1;
                result("accepted")
            }
        };
        Some(next)
    });
    (handler, ledger)
}

#[test]
fn platform_orders_a_revocation_after_the_assignment_it_revokes() {
    block_on(async {
        let pki = pki();
        let (handler, ledger) = ordering_platform(true);
        let (port, seen) = serve(&pki, handler).await;
        let d = descriptor(&pki, port, &pki.authority).unwrap();
        // An assignment at G-1, then the revocation at G.
        let report = sa::report(&d, &assignment(3), fast()).await;
        assert_eq!(report.result, Ok(Delivery::Accepted));
        let report = sa::report_revocation(&d, &revocation(4), fast()).await;
        assert_eq!(
            (report.result, report.attempts),
            (Ok(Delivery::Accepted), 1)
        );
        {
            let seen = seen.lock().unwrap();
            assert_eq!(seen[1].0, sa::REVOCATION_PATH);
            assert_eq!(seen[1].1, REVOCATION_FIXTURE.as_bytes());
        }
        let changes = ledger.lock().unwrap().changes;
        assert_eq!(changes, 2);
        // A byte-identical replay of the latest revocation: accepted, no
        // state change.
        let report = sa::report_revocation(&d, &revocation(4), fast()).await;
        assert_eq!(report.result, Ok(Delivery::Accepted));
        assert_eq!(ledger.lock().unwrap().changes, changes);
        // Lower generations are superseded.
        let report = sa::report_revocation(&d, &revocation(3), fast()).await;
        assert_eq!(report.result, Ok(Delivery::Superseded));
        let report = sa::report(&d, &assignment(3), fast()).await;
        assert_eq!(report.result, Ok(Delivery::Superseded));
        // An assignment, or another revocation time, at the revocation's key
        // conflicts; `409` is definite.
        let report = sa::report(&d, &assignment(4), fast()).await;
        assert_eq!(
            (report.result, report.attempts),
            (Err(NotDelivered::Conflict), 1)
        );
        let mut other = revocation(4);
        other.revoked_at += 1;
        let report = sa::report_revocation(&d, &other, fast()).await;
        assert_eq!(
            (report.result, report.attempts),
            (Err(NotDelivered::Conflict), 1)
        );
        let ledger = ledger.lock().unwrap();
        assert_eq!(ledger.changes, changes);
        let latest = ledger.latest.values().next().unwrap();
        assert_eq!(latest.2, REVOCATION_FIXTURE.as_bytes());
    });
}

#[test]
fn a_revocation_conflicts_with_an_assignment_at_its_key() {
    block_on(async {
        let pki = pki();
        let (handler, ledger) = ordering_platform(true);
        let (port, _) = serve(&pki, handler).await;
        let d = descriptor(&pki, port, &pki.authority).unwrap();
        let report = sa::report(&d, &assignment(4), fast()).await;
        assert_eq!(report.result, Ok(Delivery::Accepted));
        let report = sa::report_revocation(&d, &revocation(4), fast()).await;
        assert_eq!(
            (report.result, report.attempts),
            (Err(NotDelivered::Conflict), 1)
        );
        // Only an assignment above the revocation makes the scope routable.
        let report = sa::report_revocation(&d, &revocation(5), fast()).await;
        assert_eq!(report.result, Ok(Delivery::Accepted));
        let report = sa::report(&d, &assignment(6), fast()).await;
        assert_eq!(report.result, Ok(Delivery::Accepted));
        assert_eq!(ledger.lock().unwrap().changes, 3);
    });
}

#[test]
fn a_platform_without_the_revocation_endpoint_stops_the_report_at_once() {
    block_on(async {
        let pki = pki();
        let (handler, ledger) = ordering_platform(false);
        let (port, seen) = serve(&pki, handler).await;
        let d = descriptor(&pki, port, &pki.authority).unwrap();
        let report = sa::report_revocation(&d, &revocation(4), fast()).await;
        assert_eq!(
            (report.result, report.attempts),
            (Err(NotDelivered::UnexpectedStatus), 1)
        );
        assert!(NotDelivered::UnexpectedStatus.definite());
        assert_eq!(seen.lock().unwrap().len(), 1);
        assert_eq!(ledger.lock().unwrap().changes, 0);
    });
}

#[test]
fn revocation_statuses_follow_the_section_4_list() {
    block_on(async {
        let pki = pki();
        let script = vec![
            answer("400 Bad Request", ""),
            answer("401 Unauthorized", ""),
            answer("409 Conflict", ""),
            answer("429 Too Many Requests", ""),
            answer("503 Service Unavailable", ""),
            answer("404 Not Found", ""),
            answer("500 Internal Server Error", ""),
            answer("200 OK", r#"{"contract_version":1,"result":"refreshed"}"#),
            answer("503 Service Unavailable", "busy"),
            // The assignment's own handling of a status outside §4 is
            // unchanged.
            answer("404 Not Found", ""),
        ];
        let (port, _) = platform(&pki, script).await;
        let d = descriptor(&pki, port, &pki.authority).unwrap();
        let capacity = TransientCapacity::new();
        let body = sa::encode_revocation(&revocation(4)).unwrap();
        for expected in [
            NotDelivered::Malformed,
            NotDelivered::Unauthenticated,
            NotDelivered::Conflict,
            NotDelivered::RateLimited,
            NotDelivered::Unavailable,
            NotDelivered::UnexpectedStatus,
            NotDelivered::UnexpectedStatus,
            NotDelivered::InvalidResponse,
            NotDelivered::InvalidResponse,
        ] {
            assert_eq!(
                sa::deliver_revocation(&d, &capacity, &body).await,
                Err(expected)
            );
        }
        let body = sa::encode(&assignment(3)).unwrap();
        assert_eq!(
            sa::deliver(&d, &capacity, &body).await,
            Err(NotDelivered::Unavailable)
        );
    });
}

#[test]
fn revocation_response_over_the_bound_is_refused_in_transport_for_every_status() {
    block_on(async {
        let pki = pki();
        let oversized = " ".repeat(sa::RESPONSE_BYTES + 1);
        let script = vec![
            answer("200 OK", &oversized),
            answer("409 Conflict", &oversized),
            answer("404 Not Found", &oversized),
            answer("200 OK", &" ".repeat(sa::RESPONSE_BYTES)),
        ];
        let (port, _) = platform(&pki, script).await;
        let d = descriptor(&pki, port, &pki.authority).unwrap();
        let capacity = TransientCapacity::new();
        let body = sa::encode_revocation(&revocation(4)).unwrap();
        for _ in 0..2 {
            assert_eq!(
                sa::deliver_revocation(&d, &capacity, &body).await,
                Err(NotDelivered::Unavailable)
            );
        }
        // A status outside §4 stays definite whatever its body.
        assert_eq!(
            sa::deliver_revocation(&d, &capacity, &body).await,
            Err(NotDelivered::UnexpectedStatus)
        );
        // At the bound the body reaches the exact decoder.
        assert_eq!(
            sa::deliver_revocation(&d, &capacity, &body).await,
            Err(NotDelivered::InvalidResponse)
        );
    });
}

#[test]
fn unexpected_status_with_an_oversized_body_stops_without_retry() {
    block_on(async {
        let pki = pki();
        let oversized = " ".repeat(sa::RESPONSE_BYTES + 1);
        let unframed = Answer {
            stall: Duration::ZERO,
            response: format!("HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n{oversized}"),
        };
        let chunked = Answer {
            stall: Duration::ZERO,
            response: format!(
                "HTTP/1.1 500 Internal Server Error\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{oversized}\r\n0\r\n\r\n",
                oversized.len()
            ),
        };
        let script = vec![answer("404 Not Found", &oversized), unframed, chunked];
        let (port, seen) = platform(&pki, script).await;
        let d = descriptor(&pki, port, &pki.authority).unwrap();
        for _ in 0..3 {
            let report = sa::report_revocation(&d, &revocation(4), fast()).await;
            assert_eq!(
                (report.result, report.attempts),
                (Err(NotDelivered::UnexpectedStatus), 1)
            );
        }
        assert_eq!(seen.lock().unwrap().len(), 3);
    });
}

#[test]
fn transient_revocation_failures_retry_with_identical_bytes() {
    block_on(async {
        let pki = pki();
        let script = vec![
            answer("503 Service Unavailable", ""),
            answer("429 Too Many Requests", ""),
            result("accepted"),
        ];
        let (port, seen) = platform(&pki, script).await;
        let d = descriptor(&pki, port, &pki.authority).unwrap();
        let report = sa::report_revocation(&d, &revocation(4), fast()).await;
        assert_eq!(
            (report.result, report.attempts),
            (Ok(Delivery::Accepted), 3)
        );
        let seen = seen.lock().unwrap();
        assert!(seen.iter().all(
            |(path, body)| path == sa::REVOCATION_PATH && body == REVOCATION_FIXTURE.as_bytes()
        ));
    });
}

#[test]
fn revocation_outside_the_grammar_is_not_sent() {
    block_on(async {
        let pki = pki();
        let (port, seen) = platform(&pki, vec![result("accepted")]).await;
        let d = descriptor(&pki, port, &pki.authority).unwrap();
        let report = sa::report_revocation(&d, &revocation(0), fast()).await;
        assert_eq!(
            (report.result, report.attempts),
            (Err(NotDelivered::InvalidReport), 0)
        );
        assert!(seen.lock().unwrap().is_empty());
    });
}
