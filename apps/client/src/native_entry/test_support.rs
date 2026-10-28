use super::*;
use crate::ClientBootstrap;
use oteryn_client_runtime::ClientRuntime;
use rustls::pki_types::PrivatePkcs8KeyDer;
use std::io::{Read as _, Write as _};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

const WORLD_REF: &str = "w-alpha";
const CHARACTER: &str = "01890a5d-ac96-774b-bcce-b302099a8058";
const WORLD: &str = "01890a5d-ac96-774b-bcce-b302099a8059";
const CHANNEL: &str = "01890a5d-ac96-774b-bcce-b302099a805a";
const JWS: &str = "eyJhbGciOiJFZERTQSJ9.eyJzdWIiOiJ4In0.c2lnbmF0dXJl";

/// A local Platform + Gateway (one response per connection) and a TLS game endpoint
/// that answers every admission with the next scripted refusal code.
pub(super) struct Stack {
    pub(super) servers: ClientRuntime,
    http_url: String,
    requests: Arc<Mutex<Vec<String>>>,
    pub(super) admissions: Arc<AtomicUsize>,
    pub(super) root_path: PathBuf,
    pub(super) game_port: u16,
}

impl Stack {
    pub(super) fn start(refusals: Vec<u32>) -> Result<Self, Box<dyn std::error::Error>> {
        let servers = ClientRuntime::new()?;
        let generated = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
        let root = generated.cert.der().clone();
        static STACKS: AtomicUsize = AtomicUsize::new(0);
        let root_path = std::env::temp_dir().join(format!(
            "oteryn-n4-root-{}-{}.der",
            std::process::id(),
            STACKS.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::write(&root_path, root.as_ref())?;
        let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
        let mut tls = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(vec![root], key.into())?;
        tls.alpn_protocols = vec![b"oteryn-game/1".to_vec()];
        let acceptor = TlsAcceptor::from(Arc::new(tls));
        let game = servers.block_on(TcpListener::bind("127.0.0.1:0"))??;
        let game_port = game.local_addr()?.port();
        let admissions = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&admissions);
        servers.spawn(async move {
            while let Ok((tcp, _peer)) = game.accept().await {
                let index = counter.fetch_add(1, Ordering::SeqCst);
                let code = refusals
                    .get(index)
                    .or(refusals.last())
                    .copied()
                    .unwrap_or(1100);
                let Ok(mut stream) = acceptor.accept(tcp).await else {
                    continue;
                };
                let _ = oteryn_session::read_frame(&mut stream).await;
                let mut frame = vec![0x08, 0x0e, 0x22, 0x05, 0x08];
                frame.extend([(code & 0x7f) as u8 | 0x80, (code >> 7) as u8]);
                frame.extend([0x10, 0x04]);
                let _ = oteryn_session::write_frame(&mut stream, &frame).await;
                let _ = stream.shutdown().await;
            }
        })?;
        let http = servers.block_on(TcpListener::bind("127.0.0.1:0"))??;
        let http_url = format!("http://127.0.0.1:{}/", http.local_addr()?.port());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&requests);
        servers.spawn(async move {
            while let Ok((stream, _peer)) = http.accept().await {
                serve_http(stream, game_port, &log).await;
            }
        })?;
        Ok(Self {
            servers,
            http_url,
            requests,
            admissions,
            root_path,
            game_port,
        })
    }

    pub(super) fn config(&self) -> NativeLoginConfig {
        NativeLoginConfig {
            platform_url: self.http_url.clone(),
            gateway_url: self.http_url.clone(),
            oauth_client_id: "oteryn-native".to_owned(),
            world_ref: WORLD_REF.to_owned(),
            character_id: CHARACTER.to_owned(),
            dev_root: self.root_path.clone(),
            client_build: "oteryn-client/test".to_owned(),
            callback_timeout: Duration::from_secs(10),
            retry_unit: Duration::from_millis(10),
            open_browser: Arc::new(complete_browser_sign_in),
        }
    }

    pub(super) fn requests(&self) -> Vec<String> {
        self.requests
            .lock()
            .map(|requests| requests.clone())
            .unwrap_or_default()
    }

    pub(super) fn entry(
        &self,
        config: NativeLoginConfig,
    ) -> Result<GameplayEntryError, Box<dyn std::error::Error>> {
        let client = ClientBootstrap::new()?.with_native_login(config);
        let result = client.request_gameplay_entry();
        client.shutdown();
        match result {
            Ok(_admitted) => Err("refusal-only server admitted".into()),
            Err(error) => Ok(error),
        }
    }

    pub(super) fn finish(self) {
        let _ = std::fs::remove_file(&self.root_path);
        self.servers.shutdown(Duration::from_millis(100));
    }
}

/// The browser: signs in at once and follows the redirect with the exact `state`.
fn complete_browser_sign_in(url: &str) -> bool {
    let query_value = |name: &str| {
        url.split(['?', '&'])
            .find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
            .map(|value| value.replace("%3A", ":").replace("%2F", "/"))
    };
    let (Some(redirect), Some(state)) = (query_value("redirect_uri"), query_value("state")) else {
        return false;
    };
    let Some(authority) = redirect
        .strip_prefix("http://")
        .and_then(|rest| rest.strip_suffix("/callback"))
        .map(str::to_owned)
    else {
        return false;
    };
    std::thread::spawn(move || {
        if let Ok(mut stream) = std::net::TcpStream::connect(&authority) {
            let _ = write!(
                stream,
                "GET /callback?code=code-1&state={state} HTTP/1.1\r\nhost: {authority}\r\n\r\n"
            );
            let _ = stream.read_to_end(&mut Vec::new());
        }
    });
    true
}

async fn serve_http(mut stream: tokio::net::TcpStream, game_port: u16, log: &Mutex<Vec<String>>) {
    let mut request = Vec::new();
    let mut chunk = [0_u8; 4096];
    let (head, body) = loop {
        let Ok(read) = stream.read(&mut chunk).await else {
            return;
        };
        if read == 0 {
            return;
        }
        request.extend_from_slice(&chunk[..read]);
        let text = String::from_utf8_lossy(&request).into_owned();
        if let Some((head, body)) = text.split_once("\r\n\r\n") {
            let length = head
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .and_then(|value| value.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            if body.len() >= length {
                break (head.to_owned(), body.to_owned());
            }
        }
    };
    let line = head.lines().next().unwrap_or_default();
    let target = line.split(' ').take(2).collect::<Vec<_>>().join(" ");
    let response = match target.as_str() {
        "GET /v1/client/directory" => format!(
            r#"{{"epoch":1,"worlds":[{{"id":"{WORLD_REF}","name":"Alpha","channels":[],"characters":[]}}]}}"#
        ),
        "POST /oauth/token" => r#"{"token_type":"Bearer","access_token":"access-1","expires_in":300,"scope":"game:ticket"}"#.to_owned(),
        "POST /v1/game-auth/tickets" => {
            r#"{"protocol_version":1,"ticket":"ticket-1","expires_in":60}"#.to_owned()
        }
        "POST /v1/login" => {
            let attempt_ref = body
                .split_once(r#""attempt_ref":""#)
                .and_then(|(_, rest)| rest.get(..36))
                .unwrap_or_default();
            format!(
                r#"{{"protocol_version":2,"attempt_ref":"{attempt_ref}","world_id":"{WORLD}","channel_id":"{CHANNEL}","endpoint":{{"host":"127.0.0.1","port":{game_port},"tls_server_name":"localhost","alpn":"oteryn-game/1","protocol_major":1,"transport_profile":1}},"grant":{{"profile":"oteryn-pre-admission-v1","token":"{JWS}","valid_for_seconds":30}}}}"#
            )
        }
        _ => String::new(),
    };
    if let Ok(mut requests) = log.lock() {
        requests.push(format!("{target} {body}"));
    }
    let status = if response.is_empty() {
        "404 Not Found"
    } else {
        "200 OK"
    };
    let _ = stream
        .write_all(
            format!(
                "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}",
                response.len()
            )
            .as_bytes(),
        )
        .await;
    let _ = stream.shutdown().await;
}

pub(super) fn count(requests: &[String], target: &str) -> usize {
    requests
        .iter()
        .filter(|request| request.starts_with(target))
        .count()
}
