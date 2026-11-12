use super::*;
use oteryn_platform_client::{PlatformClientConfig, native_login::NativeLoginClient};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tokio_rustls::TlsAcceptor;
pub(super) type TestResult = Result<(), Box<dyn std::error::Error>>;
pub(super) const CLIENT: &str = "019a5f4a-1000-7000-8000-000000000001";
pub(super) struct Server {
    pub client: NativeLoginClient,
    pub requests: Arc<std::sync::Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<Result<(), String>>,
}
impl Server {
    pub async fn start(replies: Vec<(u16, Vec<u8>)>) -> Result<Self, Box<dyn std::error::Error>> {
        Self::start_inner(replies, false).await
    }
    pub async fn start_with_delayed_ticket(
        replies: Vec<(u16, Vec<u8>)>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::start_inner(replies, true).await
    }
    async fn start_inner(
        replies: Vec<(u16, Vec<u8>)>,
        delayed_ticket: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
        let root = certificate.cert.der().clone();
        let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(
            vec![root.clone()],
            rustls::pki_types::PrivatePkcs8KeyDer::from(certificate.signing_key.serialize_der())
                .into(),
        )?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let config = PlatformClientConfig::new(&format!(
            "https://localhost:{}",
            listener.local_addr()?.port()
        ))?;
        let client = NativeLoginClient::new(&config, &config, CLIENT)?
            .with_additional_tls_root(root.as_ref())?;
        let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = requests.clone();
        let task = tokio::spawn(async move {
            let acceptor = TlsAcceptor::from(Arc::new(tls));
            for (status, mut body) in replies {
                let (socket, _) = listener.accept().await.map_err(|_| "accept")?;
                let mut stream = acceptor.accept(socket).await.map_err(|_| "TLS")?;
                let mut request = Vec::new();
                let mut chunk = [0_u8; 4096];
                loop {
                    let n = stream.read(&mut chunk).await.map_err(|_| "read")?;
                    if n == 0 || request.len() + n > 32_768 {
                        return Err("request incomplete".into());
                    }
                    request.extend_from_slice(&chunk[..n]);
                    let text = String::from_utf8_lossy(&request);
                    if let Some((header, body)) = text.split_once("\r\n\r\n") {
                        let len = header
                            .lines()
                            .find_map(|line| {
                                line.split_once(':')
                                    .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                                    .and_then(|(_, v)| v.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        if body.len() >= len {
                            break;
                        }
                    }
                }
                log.lock()
                    .map_err(|_| "log")?
                    .push(String::from_utf8_lossy(&request).into_owned());
                if delayed_ticket
                    && request.starts_with(b"POST /api/v1/game-auth/device-sessions/tickets ")
                {
                    let issued_at = next_utc_second().await?;
                    let mut reply: serde_json::Value =
                        serde_json::from_slice(&body).map_err(|_| "ticket fixture")?;
                    reply["expires_at"] = (issued_at + 60).into();
                    body = serde_json::to_vec(&reply).map_err(|_| "ticket fixture")?;
                }
                let header = format!(
                    "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nCache-Control: no-store, private\r\nPragma: no-cache\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream
                    .write_all(header.as_bytes())
                    .await
                    .map_err(|_| "header")?;
                stream.write_all(&body).await.map_err(|_| "body")?;
                let _ = stream.shutdown().await;
            }
            Ok(())
        });
        Ok(Self {
            client,
            requests,
            task,
        })
    }
    pub async fn finish(self) -> TestResult {
        self.task.await?.map_err(|e| e.into())
    }
}

async fn next_utc_second() -> Result<u64, String> {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    let boundary = Duration::from_secs(now().map_err(|_| "clock")? + 1);
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let utc = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| "clock")?;
            if utc >= boundary {
                return Ok(utc.as_secs());
            }
            tokio::time::sleep(boundary - utc).await;
        }
    })
    .await
    .map_err(|_| "clock boundary timeout")?
}

pub(super) fn fixture(name: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/contracts/fixtures/remembered-device-v1")
            .join(name),
    )?)
}
pub(super) fn changed_response(
    credential: &str,
    ticket: bool,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut body: serde_json::Value = serde_json::from_slice(&if ticket {
        fixture("ticket-response.json")?
    } else {
        enrollment_response()?
    })?;
    let epoch = fixture_epoch();
    body["absolute_expires_at"] = (epoch + 30 * 86400).into();
    body["idle_expires_at"] = (epoch + 7 * 86400).into();
    body["device_credential"] = credential.into();
    if ticket {
        body["expires_at"] = (now()? + 60).into();
    } else {
        body["characters"] = serde_json::json!([{"character_id":"01890a5d-ac96-774b-bcce-b302099a8058","world_id":"01890a5d-ac96-774b-bcce-b302099a8059","name":"Fixture Knight","availability":"AVAILABLE"}]);
    }
    Ok(serde_json::to_vec(&body)?)
}

// Preserve the signed fixture bytes on disk; move only HTTP test expiry values to
// a per-test-process epoch so qualification does not expire with the sample data.
fn fixture_epoch() -> u64 {
    static EPOCH: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *EPOCH.get_or_init(|| now().unwrap_or(0))
}
pub(super) fn enrollment_response() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut body: serde_json::Value = serde_json::from_slice(&fixture("enroll-response.json")?)?;
    let epoch = fixture_epoch();
    body["absolute_expires_at"] = (epoch + 30 * 86400).into();
    body["idle_expires_at"] = (epoch + 7 * 86400).into();
    Ok(serde_json::to_vec(&body)?)
}
