//! PREM-1b: the in-process Premium test producer (PREMIUM-DELIVERY-0 §3.1; §11 scope item 5).
//!
//! A mutual-TLS HTTP/1.1 server on loopback that speaks exactly the §3.1 exchange: it serves
//! only `POST /v1/premium/snapshot` with a §3.1 request body from a client certificate issued by
//! its own test CA, and answers each request from a scriptable handler (status, headers, body,
//! delay; a wrong nonce or account is a body the handler writes). [`snapshot`] builds the §4
//! body. Test-only: compiled under `cfg(test)` and by the PREM-1b integration test, never into
//! the production binary. Every certificate is generated here for the test only.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// One test PKI: a CA, the producer's server certificate for `127.0.0.1`, and a Game client
/// identity, both issued by the CA.
pub struct TestPki {
    pub ca_pem: String,
    /// PEM certificate and PKCS#8 key of the Game client.
    pub client_identity_pem: String,
    server_chain: Vec<CertificateDer<'static>>,
    server_key: PrivateKeyDer<'static>,
    ca_der: CertificateDer<'static>,
}

impl TestPki {
    pub fn new() -> Self {
        let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let ca = CertifiedIssuer::self_signed(params, KeyPair::generate().unwrap()).unwrap();
        let issue = |name: &str, usage| {
            let key = KeyPair::generate().unwrap();
            let mut params = CertificateParams::new(vec![name.to_owned()]).unwrap();
            params.extended_key_usages = vec![usage];
            (params.signed_by(&key, &ca).unwrap(), key)
        };
        let (server, server_key) = issue("127.0.0.1", ExtendedKeyUsagePurpose::ServerAuth);
        let (client, client_key) = issue(
            "game-node.premium-snapshot",
            ExtendedKeyUsagePurpose::ClientAuth,
        );
        Self {
            ca_pem: pem("CERTIFICATE", ca.der()),
            client_identity_pem: format!(
                "{}{}",
                pem("CERTIFICATE", client.der()),
                pem("PRIVATE KEY", &client_key.serialize_der())
            ),
            server_chain: vec![server.der().clone()],
            server_key: PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(server_key.serialize_der())),
            ca_der: ca.der().clone(),
        }
    }
}

impl Default for TestPki {
    fn default() -> Self {
        Self::new()
    }
}

fn pem(label: &str, der: &[u8]) -> String {
    use base64::Engine;
    let text = base64::engine::general_purpose::STANDARD.encode(der);
    let lines: Vec<&str> = text
        .as_bytes()
        .chunks(64)
        .map(|line| std::str::from_utf8(line).unwrap())
        .collect();
    format!(
        "-----BEGIN {label}-----\n{}\n-----END {label}-----\n",
        lines.join("\n")
    )
}

/// One request the producer received: its parsed §3.1 body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProducerRequest {
    pub account_id: String,
    pub nonce: String,
}

/// One scripted answer.
#[derive(Debug, Clone)]
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub delay: Duration,
}

impl Reply {
    /// A 200 with `application/json`.
    pub fn json(body: Vec<u8>) -> Self {
        Self {
            status: 200,
            headers: vec![("Content-Type".into(), "application/json".into())],
            body,
            delay: Duration::ZERO,
        }
    }

    pub fn status(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
            delay: Duration::ZERO,
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn delayed(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

type Handler = Arc<dyn Fn(&ProducerRequest) -> Reply + Send + Sync>;

/// The running producer. It stops when dropped.
pub struct TestProducer {
    pub origin: String,
    handler: Arc<Mutex<Handler>>,
    requests: Arc<Mutex<Vec<ProducerRequest>>>,
    max_in_flight: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for TestProducer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl TestProducer {
    /// Start on an ephemeral loopback port. Must run inside the Tokio runtime.
    pub async fn start(
        pki: &TestPki,
        handler: impl Fn(&ProducerRequest) -> Reply + Send + Sync + 'static,
    ) -> Self {
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let mut roots = rustls::RootCertStore::empty();
        roots.add(pki.ca_der.clone()).unwrap();
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
            .with_single_cert(pki.server_chain.clone(), pki.server_key.clone_key())
            .unwrap();
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let handler: Arc<Mutex<Handler>> = Arc::new(Mutex::new(Arc::new(handler)));
        let requests = Arc::<Mutex<Vec<ProducerRequest>>>::default();
        let max_in_flight = Arc::<AtomicUsize>::default();
        let in_flight = Arc::<AtomicUsize>::default();
        let (shared, seen, max) = (handler.clone(), requests.clone(), max_in_flight.clone());
        let task = tokio::spawn(async move {
            while let Ok((tcp, _)) = listener.accept().await {
                let (acceptor, shared, seen) = (acceptor.clone(), shared.clone(), seen.clone());
                let (in_flight, max) = (in_flight.clone(), max.clone());
                tokio::spawn(async move {
                    let Ok(mut tls) = acceptor.accept(tcp).await else {
                        return;
                    };
                    let Some(request) = read_request(&mut tls).await else {
                        let _ = tls.write_all(&response(&Reply::status(400))).await;
                        return;
                    };
                    let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    max.fetch_max(now, Ordering::SeqCst);
                    seen.lock().unwrap().push(request.clone());
                    let handler = shared.lock().unwrap().clone();
                    let reply = handler(&request);
                    tokio::time::sleep(reply.delay).await;
                    let _ = tls.write_all(&response(&reply)).await;
                    let _ = tls.shutdown().await;
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });
        Self {
            origin: format!("https://127.0.0.1:{port}"),
            handler,
            requests,
            max_in_flight,
            task,
        }
    }

    pub fn script(&self, handler: impl Fn(&ProducerRequest) -> Reply + Send + Sync + 'static) {
        *self.handler.lock().unwrap() = Arc::new(handler);
    }

    pub fn requests(&self) -> Vec<ProducerRequest> {
        self.requests.lock().unwrap().clone()
    }

    pub fn max_in_flight(&self) -> usize {
        self.max_in_flight.load(Ordering::SeqCst)
    }
}

/// Read one §3.1 request; `None` unless it is exactly `POST /v1/premium/snapshot` with
/// `application/json` and a §3.1 body.
async fn read_request<S: tokio::io::AsyncRead + Unpin>(stream: &mut S) -> Option<ProducerRequest> {
    let mut head = Vec::new();
    let mut byte = [0u8];
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() > 8 * 1024 || stream.read(&mut byte).await.ok()? != 1 {
            return None;
        }
        head.push(byte[0]);
    }
    let head = String::from_utf8(head).ok()?;
    let mut lines = head.split("\r\n");
    if lines.next()? != "POST /v1/premium/snapshot HTTP/1.1" {
        return None;
    }
    let header = |name: &str| {
        head.split("\r\n").skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case(name)
                .then(|| value.trim().to_owned())
        })
    };
    if header("content-type")? != "application/json" {
        return None;
    }
    let length: usize = header("content-length")?.parse().ok()?;
    if length > oteryn_game_server::premium::client::MAX_REQUEST_BYTES {
        return None;
    }
    let mut body = vec![0; length];
    stream.read_exact(&mut body).await.ok()?;
    let value: serde_json::Map<String, serde_json::Value> = serde_json::from_slice(&body).ok()?;
    let text = |key: &str| value.get(key)?.as_str().map(str::to_owned);
    let nonce = text("nonce")?;
    (value.len() == 3
        && text("schema")?.as_str() == oteryn_game_server::premium::client::REQUEST_SCHEMA
        && nonce.len() == 32
        && nonce
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
    .then(|| ProducerRequest {
        account_id: text("account_id").unwrap_or_default(),
        nonce,
    })
}

fn response(reply: &Reply) -> Vec<u8> {
    let mut out = format!(
        "HTTP/1.1 {} Scripted\r\nContent-Length: {}\r\nConnection: close\r\n",
        reply.status,
        reply.body.len()
    );
    for (name, value) in &reply.headers {
        out.push_str(&format!("{name}: {value}\r\n"));
    }
    out.push_str("\r\n");
    let mut bytes = out.into_bytes();
    bytes.extend_from_slice(&reply.body);
    bytes
}

/// The §4 body of an `ACTIVE` snapshot answering `request`, with `overrides` applied (a
/// `serde_json::Value::Null` override sets the member to null).
pub fn snapshot(
    request: &ProducerRequest,
    authority_revision: u64,
    overrides: &[(&str, serde_json::Value)],
) -> Vec<u8> {
    let mut wire = serde_json::json!({
        "schema": "oteryn.premium_snapshot.v1",
        "producer_revision": "c914564",
        "producer_profile": "oteryn.entitlement.profile_b.v1",
        "nonce": request.nonce,
        "account_id": request.account_id,
        "product_id": "oteryn.premium_time",
        "product_version": 1,
        "entitlement_id": "ent-1",
        "entitlement_state": "ACTIVE",
        "lifecycle_revision": 1,
        "authority_revision": authority_revision,
        "effective_from": "2026-01-01T00:00:00Z",
        "effective_until": "2099-01-01T00:00:00Z",
        "authority_issued_at": "2026-09-30T12:00:00Z",
        "authority_valid_until": "2026-09-30T13:00:00Z",
        "refresh_after": "2026-09-30T12:40:00Z",
    });
    for (key, value) in overrides {
        wire[*key] = value.clone();
    }
    serde_json::to_vec(&wire).unwrap()
}
