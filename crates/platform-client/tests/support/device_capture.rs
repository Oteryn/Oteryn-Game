//! Test-only TLS forwarding capture. Every response comes from the real Platform.
//! No credential values are stored in the capture proof or exposed in diagnostics.
use super::{devices, support::*};
use oteryn_platform_client::{
    PlatformClientConfig, device_sessions::DeviceCredential, native_login::NativeLoginClient,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::mpsc,
};
use tokio_rustls::TlsAcceptor;

const BASE: &str = "/api/v1/game-auth/device-sessions";
const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/contracts/fixtures/remembered-device-v1/"
);

pub struct RequestProof {
    enrollment: bool,
    valid: bool,
}
impl RequestProof {
    pub fn require(&self, enrollment: bool) -> TestResult {
        check(
            self.enrollment == enrollment && self.valid,
            "shipping_request_does_not_match_contract",
        )
    }
}

pub struct RequestMeta<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub content_type: &'a str,
    pub content_encoding: Option<&'a str>,
    pub authorization: &'a str,
}

pub fn contract_request(
    meta: &RequestMeta<'_>,
    body: &[u8],
    enrollment: bool,
    client: &str,
    issued: Option<&DeviceCredential>,
) -> TestResult<bool> {
    let expected = if enrollment {
        std::fs::read(format!("{FIXTURES}enroll.json")).map_err(|_| "contract_fixture_missing")?
    } else {
        let fixture = std::fs::read_to_string(format!("{FIXTURES}rotate.json"))
            .map_err(|_| "contract_fixture_missing")?;
        fixture
            .replace("019a5f4a-1000-7000-8000-000000000001", client)
            .into_bytes()
    };
    let authorized = if enrollment {
        meta.authorization
            .strip_prefix("Bearer ")
            .is_some_and(|value| !value.is_empty() && value.bytes().all(|b| b.is_ascii_graphic()))
    } else {
        meta.authorization
            .strip_prefix("OterynDevice ")
            .and_then(|value| DeviceCredential::parse(value).ok())
            .is_some_and(|credential| issued == Some(&credential))
    };
    Ok(meta.method == "POST"
        && meta.path
            == if enrollment {
                BASE.to_owned()
            } else {
                format!("{BASE}/native-characters")
            }
        && meta.content_type == "application/json"
        && matches!(meta.content_encoding, None | Some("identity"))
        && authorized
        && body == expected)
}

pub struct Capture {
    requests: mpsc::Receiver<RequestProof>,
    task: tokio::task::JoinHandle<TestResult>,
    issued: Arc<Mutex<Option<DeviceCredential>>>,
}
impl Capture {
    pub async fn start(t: &Topology) -> TestResult<(Self, NativeLoginClient)> {
        let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .map_err(|_| "capture_certificate_failed")?;
        let der = certificate.cert.der().clone();
        let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|_| "capture_tls_failed")?
        .with_no_client_auth()
        .with_single_cert(
            vec![der.clone()],
            rustls::pki_types::PrivatePkcs8KeyDer::from(certificate.signing_key.serialize_der())
                .into(),
        )
        .map_err(|_| "capture_tls_failed")?;
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|_| "capture_bind_failed")?;
        let origin = format!(
            "https://localhost:{}",
            listener
                .local_addr()
                .map_err(|_| "capture_address_failed")?
                .port()
        );
        let native = NativeLoginClient::new(
            &PlatformClientConfig::new(&origin).map_err(|_| "capture_origin_invalid")?,
            &PlatformClientConfig::new(text(&t.report["baseUrls"], "gateway")?)
                .map_err(|_| "gateway_origin_invalid")?,
            &t.input.client,
        )
        .map_err(|_| "capture_client_failed")?
        .with_additional_tls_root(der.as_ref())
        .map_err(|_| "capture_root_failed")?;
        let upstream = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .add_root_certificate(
                reqwest::Certificate::from_pem(&devices::ca(t)?)
                    .map_err(|_| "capture_upstream_ca_invalid")?,
            )
            .build()
            .map_err(|_| "capture_upstream_client_failed")?;
        let upstream_origin = t.input.native_origin.clone();
        let client = t.input.client.clone();
        let (sender, requests) = mpsc::channel(2);
        let issued = Arc::new(Mutex::new(None));
        let expected_credential = issued.clone();
        let task = tokio::spawn(async move {
            let acceptor = TlsAcceptor::from(Arc::new(tls));
            let mut count = 0;
            loop {
                let (socket, _) = listener
                    .accept()
                    .await
                    .map_err(|_| "capture_accept_failed")?;
                let mut stream = acceptor
                    .accept(socket)
                    .await
                    .map_err(|_| "capture_tls_failed")?;
                let mut bytes = Vec::new();
                let mut chunk = [0; 2048];
                let (headers_end, length) = loop {
                    let size = stream
                        .read(&mut chunk)
                        .await
                        .map_err(|_| "capture_read_failed")?;
                    check(
                        size > 0 && bytes.len() + size <= 16384,
                        "capture_request_bounds",
                    )?;
                    bytes.extend_from_slice(&chunk[..size]);
                    if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = std::str::from_utf8(&bytes[..end])
                            .map_err(|_| "capture_headers_invalid")?;
                        let lengths: Vec<_> = headers
                            .lines()
                            .skip(1)
                            .filter_map(|line| line.split_once(':'))
                            .filter(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                            .collect();
                        check(lengths.len() == 1, "capture_content_length_invalid")?;
                        let length = lengths[0]
                            .1
                            .trim()
                            .parse::<usize>()
                            .map_err(|_| "capture_content_length_invalid")?;
                        check(length <= 1024, "capture_body_bounds")?;
                        break (end + 4, length);
                    }
                };
                while bytes.len() < headers_end + length {
                    let size = stream
                        .read(&mut chunk)
                        .await
                        .map_err(|_| "capture_read_failed")?;
                    check(
                        size > 0 && bytes.len() + size <= headers_end + length,
                        "capture_body_bounds",
                    )?;
                    bytes.extend_from_slice(&chunk[..size]);
                }
                check(
                    bytes.len() == headers_end + length,
                    "capture_trailing_request_bytes",
                )?;
                let headers = std::str::from_utf8(&bytes[..headers_end - 4])
                    .map_err(|_| "capture_headers_invalid")?;
                let mut lines = headers.lines();
                let start: Vec<_> = lines
                    .next()
                    .ok_or("capture_start_missing")?
                    .split(' ')
                    .collect();
                check(
                    start.len() == 3 && start[2] == "HTTP/1.1",
                    "capture_start_invalid",
                )?;
                let method = start[0];
                let path = start[1];
                let mut authorization = None;
                let mut content_type = None;
                let mut content_encoding = None;
                let mut request = upstream.request(
                    reqwest::Method::from_bytes(method.as_bytes())
                        .map_err(|_| "capture_method_invalid")?,
                    format!("{upstream_origin}{path}"),
                );
                check(
                    path.starts_with(BASE) && !path.contains(['?', '#']),
                    "capture_path_invalid",
                )?;
                for line in lines {
                    let (key, value) = line.split_once(':').ok_or("capture_header_invalid")?;
                    let value = value.trim();
                    if key.eq_ignore_ascii_case("authorization") {
                        check(authorization.is_none(), "capture_duplicate_authorization")?;
                        authorization = Some(value);
                    }
                    if key.eq_ignore_ascii_case("content-type") {
                        check(content_type.is_none(), "capture_duplicate_content_type")?;
                        content_type = Some(value);
                    }
                    if key.eq_ignore_ascii_case("content-encoding") {
                        check(
                            content_encoding.is_none(),
                            "capture_duplicate_content_encoding",
                        )?;
                        content_encoding = Some(value);
                    }
                    if !["host", "content-length", "connection"]
                        .iter()
                        .any(|name| key.eq_ignore_ascii_case(name))
                    {
                        let mut header = reqwest::header::HeaderValue::from_str(value)
                            .map_err(|_| "capture_header_invalid")?;
                        header.set_sensitive(true);
                        request = request.header(key, header);
                    }
                }
                let authorization = authorization.ok_or("capture_authorization_missing")?;
                let (_, secret) = authorization
                    .split_once(' ')
                    .ok_or("capture_authorization_invalid")?;
                check(
                    !secret.is_empty() && secret.bytes().all(|b| b.is_ascii_graphic()),
                    "capture_authorization_invalid",
                )?;
                if count < 2 {
                    let enrollment = count == 0;
                    let issued = expected_credential
                        .lock()
                        .map_err(|_| "capture_expected_credential_failed")?
                        .clone();
                    let valid = contract_request(
                        &RequestMeta {
                            method,
                            path,
                            content_type: content_type.ok_or("capture_content_type_missing")?,
                            content_encoding,
                            authorization,
                        },
                        &bytes[headers_end..],
                        enrollment,
                        &client,
                        issued.as_ref(),
                    )?;
                    sender
                        .send(RequestProof { enrollment, valid })
                        .await
                        .map_err(|_| "capture_receiver_closed")?;
                }
                count += 1;
                // Forward the captured body verbatim. Never construct a success or
                // refusal: status, headers and body are the real upstream response.
                let response = request
                    .body(bytes[headers_end..].to_vec())
                    .send()
                    .await
                    .map_err(|_| "capture_upstream_failed")?;
                let status = response.status();
                let mut output = format!(
                    "HTTP/1.1 {} {}\r\n",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or("")
                );
                for (key, value) in response.headers() {
                    if !["connection", "transfer-encoding", "content-length"]
                        .contains(&key.as_str())
                    {
                        output.push_str(key.as_str());
                        output.push_str(": ");
                        output.push_str(
                            value
                                .to_str()
                                .map_err(|_| "capture_response_header_invalid")?,
                        );
                        output.push_str("\r\n");
                    }
                }
                check(
                    response.content_length().is_none_or(|n| n <= 16384),
                    "capture_response_bounds",
                )?;
                let body = response
                    .bytes()
                    .await
                    .map_err(|_| "capture_response_failed")?;
                check(body.len() <= 16384, "capture_response_bounds")?;
                output.push_str(&format!(
                    "Content-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                ));
                stream
                    .write_all(output.as_bytes())
                    .await
                    .map_err(|_| "capture_write_failed")?;
                stream
                    .write_all(&body)
                    .await
                    .map_err(|_| "capture_write_failed")?;
                stream
                    .shutdown()
                    .await
                    .map_err(|_| "capture_shutdown_failed")?;
            }
        });
        Ok((
            Self {
                requests,
                task,
                issued,
            },
            native,
        ))
    }
    pub fn expect_device(&mut self, credential: &DeviceCredential) -> TestResult {
        *self
            .issued
            .lock()
            .map_err(|_| "capture_expected_credential_failed")? = Some(credential.clone());
        Ok(())
    }
    pub async fn request(&mut self) -> TestResult<RequestProof> {
        tokio::time::timeout(Duration::from_secs(5), self.requests.recv())
            .await
            .map_err(|_| "capture_proof_timeout")?
            .ok_or_else(|| "capture_proof_missing".into())
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
