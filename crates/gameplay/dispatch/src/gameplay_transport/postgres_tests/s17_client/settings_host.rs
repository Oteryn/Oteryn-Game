//! Test-only mTLS preferences producer for the accepted Platform handoff.
use super::super::TestResult;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
pub(super) async fn host(
    node: [u8; 16],
) -> TestResult<(
    Arc<oteryn_gameplay_social::cast::access::CastAccessClient>,
    tokio::task::JoinHandle<()>,
)> {
    use rcgen::{BasicConstraints, CertificateParams, CertifiedIssuer, IsCa, KeyPair};
    use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
    let mut params = CertificateParams::new(Vec::<String>::new())?;
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca = CertifiedIssuer::self_signed(params, KeyPair::generate()?)?;
    let server_key = KeyPair::generate()?;
    let server = CertificateParams::new(vec!["127.0.0.1".into()])?.signed_by(&server_key, &ca)?;
    let client_key = KeyPair::generate()?;
    let client = CertificateParams::new(vec!["game.test".into()])?.signed_by(&client_key, &ca)?;
    let mut roots = rustls::RootCertStore::empty();
    roots.add(ca.der().clone())?;
    let verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(roots)).build()?;
    let config = rustls::ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(
            vec![server.der().clone()],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(server_key.serialize_der())),
        )?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("https://{}/", listener.local_addr()?);
    let identity = format!(
        "{}{}",
        pem("CERTIFICATE", client.der()),
        pem("PRIVATE KEY", &client_key.serialize_der())
    );
    let access = oteryn_gameplay_social::cast::access::CastAccessClient::new(
        &base,
        node,
        identity.as_bytes(),
        pem("CERTIFICATE", ca.der()).as_bytes(),
    )?;
    let task = tokio::spawn(async move {
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        while let Ok((socket, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                let Ok(mut stream) = acceptor.accept(socket).await else {
                    return;
                };
                let mut request = Vec::new();
                loop {
                    let mut bytes = [0; 2048];
                    let Ok(n) = stream.read(&mut bytes).await else {
                        return;
                    };
                    if n == 0 || request.len() + n > 8192 {
                        return;
                    }
                    request.extend_from_slice(&bytes[..n]);
                    let Some(end) = request.windows(4).position(|v| v == b"\r\n\r\n") else {
                        continue;
                    };
                    let Ok(header) = std::str::from_utf8(&request[..end]) else {
                        return;
                    };
                    let length = header.lines().find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|n| n.parse::<usize>().ok())
                    });
                    if length.is_some_and(|n| request.len() >= end + 4 + n) {
                        break;
                    }
                }
                let body = r#"{"delaySeconds":0,"viewerCap":50,"chatEnabled":true,"passwordRequired":false,"accessRevision":1,"description":"Test cast"}"#;
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(reply.as_bytes()).await;
                let _ = stream.shutdown().await;
            });
        }
    });
    Ok((Arc::new(access), task))
}

fn pem(label: &str, bytes: &[u8]) -> String {
    use base64::Engine;
    let body = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("-----BEGIN {label}-----\n{body}\n-----END {label}-----\n")
}
