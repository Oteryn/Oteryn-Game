//! Test-only mTLS preferences producer for the accepted Platform handoff.
use super::super::TestResult;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
pub(in super::super) async fn host_with_policy(
    node: [u8; 16],
    delay: u16,
    chat: bool,
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
    let script = Arc::new(std::sync::Mutex::new(Script::default()));
    let task = tokio::spawn(async move {
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        while let Ok((socket, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            let script = script.clone();
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
                let Some(end) = request.windows(4).position(|v| v == b"\r\n\r\n") else {
                    return;
                };
                let value = serde_json::from_slice::<serde_json::Value>(&request[end + 4..]);
                let (status, body) = match (value, script.lock()) {
                    (Ok(value), Ok(mut script)) => script.reply(&value, node, delay, chat),
                    _ => (503, serde_json::json!({})),
                };
                let body = body.to_string();
                let reply = format!(
                    "HTTP/1.1 {status} Result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
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

pub(in super::super) async fn host(
    node: [u8; 16],
) -> TestResult<(
    Arc<oteryn_gameplay_social::cast::access::CastAccessClient>,
    tokio::task::JoinHandle<()>,
)> {
    host_with_policy(node, 0, false).await
}
#[derive(Default)]
struct Script {
    announced: Option<serde_json::Value>,
    used: std::collections::BTreeSet<String>,
    reservations: std::collections::BTreeMap<String, (u8, String, u64, u64)>,
    ordinal: u8,
}
impl Script {
    fn reply(
        &mut self,
        value: &serde_json::Value,
        node: [u8; 16],
        delay: u16,
        chat: bool,
    ) -> (u16, serde_json::Value) {
        use serde_json::json;
        let unavailable = || (404, json!({}));
        match value["operation"].as_str().unwrap_or_default() {
            "preferences" => (
                200,
                json!({"delaySeconds":delay,"viewerCap":50,"chatEnabled":true,"passwordRequired":false,"accessRevision":1,"description":"Test cast"}),
            ),
            "announce" => {
                self.announced = Some(value.clone());
                (200, json!({}))
            }
            "stop" => {
                self.announced = None;
                self.reservations.clear();
                (200, json!({}))
            }
            "release" => {
                if let Some(id) = value["reservation"].as_str() {
                    self.reservations.remove(id);
                }
                (200, json!({}))
            }
            "consume" | "renew" => {
                let Some(announced) = self.announced.as_ref() else {
                    return unavailable();
                };
                let Some(cast) = announced["cast"].as_str() else {
                    return unavailable();
                };
                let Some(epoch) = announced["epoch"].as_u64() else {
                    return unavailable();
                };
                let Some(access) = announced["accessRevision"].as_u64() else {
                    return unavailable();
                };
                let (marker, reservation) = if value["operation"] == "consume" {
                    let Some(ticket) = value["ticket"].as_str() else {
                        return unavailable();
                    };
                    if ticket.len() != 69 || !ticket.is_ascii() || self.used.contains(ticket) {
                        return unavailable();
                    }
                    let Ok(marker) = u8::from_str_radix(&ticket[5..7], 16) else {
                        return unavailable();
                    };
                    if !(1..=8).contains(&marker) || self.ordinal >= 100 {
                        return unavailable();
                    }
                    self.ordinal += 1;
                    let reservation = uuid(super::super::fixture::id(120 + self.ordinal));
                    self.used.insert(ticket.to_owned());
                    self.reservations.insert(
                        reservation.clone(),
                        (marker, cast.to_owned(), epoch, access),
                    );
                    (marker, reservation)
                } else {
                    let Some(reservation) = value["reservation"].as_str() else {
                        return unavailable();
                    };
                    let Some((marker, held_cast, held_epoch, held_access)) =
                        self.reservations.get(reservation)
                    else {
                        return unavailable();
                    };
                    if held_cast != cast || *held_epoch != epoch || *held_access != access {
                        return unavailable();
                    }
                    (*marker, reservation.to_owned())
                };
                let unix = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                (
                    200,
                    json!({"purpose":"oteryn-cast-observer-v1","node":uuid(node),"account":uuid(super::super::fixture::id(80+marker)),"cast":cast,"epoch":epoch,"accessRevision":access,"reservation":reservation,"expiresAt":unix+5,"chat":chat}),
                )
            }
            _ => unavailable(),
        }
    }
}
fn uuid(bytes: [u8; 16]) -> String {
    use std::fmt::Write;
    let mut text = String::with_capacity(36);
    for (i, byte) in bytes.into_iter().enumerate() {
        if [4, 6, 8, 10].contains(&i) {
            text.push('-');
        }
        let _ = write!(text, "{byte:02x}");
    }
    text
}
