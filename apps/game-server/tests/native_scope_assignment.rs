//! `ReportScopeAssignmentV1` (`oteryn-game-native-runtime-status-v1` §5)
//! producer tests against a loopback Platform stub. Every certificate is
//! generated here for the test only.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use oteryn_game_server::native_admission_source::scope_assignment::{
    self as sa, Assignment, Delivery, NotDelivered, ReportConfig, RetryPolicy,
    ScopeAssignmentDescriptor,
};
use oteryn_game_server::native_admission_source::{SourceError, TransientCapacity};
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::collections::VecDeque;
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

/// Platform stub: only the ownership-authority identity may report (else
/// `401`). Each authenticated request takes the next scripted answer and is
/// recorded as `(path, raw body)`.
async fn platform(pki: &Pki, script: Vec<Answer>) -> (u16, Seen) {
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
    let script = Arc::new(Mutex::new(VecDeque::from(script)));
    let (allowed, record) = (pki.authority.chain[0].clone(), seen.clone());
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let Ok(mut tls) = acceptor.accept(tcp).await else {
                continue;
            };
            let (allowed, record, script) = (allowed.clone(), record.clone(), script.clone());
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
                    record.lock().unwrap().push((path, body));
                    script.lock().unwrap().pop_front()
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
        let script = vec![
            answer("200 OK", &oversized),
            answer("409 Conflict", &oversized),
            answer("200 OK", &" ".repeat(sa::RESPONSE_BYTES)),
        ];
        let (port, _) = platform(&pki, script).await;
        let d = descriptor(&pki, port, &pki.authority).unwrap();
        let capacity = TransientCapacity::new();
        let body = sa::encode(&assignment(3)).unwrap();
        for _ in 0..2 {
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
