//! `ListCharactersForAccount` projection transport against a loopback
//! Platform stub: the contractual maximum snapshot crosses the mTLS exchange.
//! Every certificate is generated here for the test only.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use oteryn_game_server::native_admission_source::account_characters::{
    AccountSnapshot, Availability, CharacterSummary, Delivery, MAX_CHARACTERS, MtlsSink,
    ProjectionDescriptor, ProjectionSink, SNAPSHOT_BYTES, encode_snapshot,
};
use oteryn_game_server::native_admission_source::descriptor::Operation;
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const PEER: &str = "platform.test";
const ACK: &str = r#"{"contract_version":1,"result":"accepted"}"#;

type Identity = (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>);

fn issue(
    ca: &CertifiedIssuer<'_, KeyPair>,
    name: &str,
    usage: ExtendedKeyUsagePurpose,
) -> Identity {
    let key = KeyPair::generate().unwrap();
    let mut params = CertificateParams::new(vec![name.to_owned()]).unwrap();
    params.extended_key_usages = vec![usage];
    let cert = params.signed_by(&key, ca).unwrap();
    (
        vec![cert.der().clone()],
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der())),
    )
}

/// Platform stub: records the body length of every request and acknowledges it.
async fn platform(
    ca: CertificateDer<'static>,
    server: Identity,
) -> (u16, Arc<Mutex<Vec<(String, usize)>>>) {
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let mut roots = rustls::RootCertStore::empty();
    roots.add(ca).unwrap();
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
        .with_single_cert(server.0, server.1)
        .unwrap();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen = Arc::<Mutex<Vec<(String, usize)>>>::default();
    let record = seen.clone();
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
            if tls.read_exact(&mut body).await.is_ok() {
                record.lock().unwrap().push((path, body.len()));
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{ACK}",
                ACK.len()
            );
            let _ = tls.write_all(response.as_bytes()).await;
            let _ = tls.shutdown().await;
        }
    });
    (port, seen)
}

#[test]
fn the_largest_snapshot_crosses_the_exchange_and_one_byte_more_is_refused() {
    let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca = CertifiedIssuer::self_signed(params, KeyPair::generate().unwrap()).unwrap();
    let client = ExtendedKeyUsagePurpose::ClientAuth;
    let server = issue(&ca, PEER, ExtendedKeyUsagePurpose::ServerAuth);
    let projection = issue(&ca, "character-authority.projection", client.clone());
    let evidence = issue(&ca, "character-authority.native-evidence", client);
    let name = "N".repeat(64);
    let characters = (0..MAX_CHARACTERS)
        .map(|i| CharacterSummary {
            character_id: format!("01934f10-7c04-7001-805b-3b11223344{i:02x}"),
            world_id: "01934f10-7c02-7001-805b-3b1122334401".into(),
            name: name.clone(),
            availability: Availability::Unavailable,
        })
        .collect();
    let largest = encode_snapshot(
        "oteryn:character-authority:primary",
        &AccountSnapshot {
            account_id: "0190f2a1-3b4c-7d5e-8f60-718293a4b5c6".into(),
            projection_epoch: u64::MAX,
            projection_revision: u64::MAX,
            source_observed_at: i64::MAX,
            characters,
        },
    )
    .unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (port, seen) = platform(ca.der().clone(), server).await;
            let descriptor = ProjectionDescriptor::new(
                ("127.0.0.1".into(), port),
                PEER.into(),
                vec![ca.der().clone()],
                projection.0,
                projection.1,
                &[&evidence.0],
            )
            .unwrap();
            let mut sink = MtlsSink::new(descriptor);
            let op = Operation::PublishAccountCharactersV1;
            // The worst-case encoded snapshot and a body of exactly
            // `LCA-REQUEST-BYTES` both cross the exchange.
            assert_eq!(sink.send(op, &largest).await, Ok(Delivery::Accepted));
            let full = " ".repeat(SNAPSHOT_BYTES);
            assert_eq!(sink.send(op, &full).await, Ok(Delivery::Accepted));
            // One byte more is refused before any connection.
            let over = " ".repeat(SNAPSHOT_BYTES + 1);
            assert!(sink.send(op, &over).await.is_err());
            let path = op.path().to_owned();
            assert_eq!(
                *seen.lock().unwrap(),
                [(path.clone(), largest.len()), (path, SNAPSHOT_BYTES)]
            );
        });
}
