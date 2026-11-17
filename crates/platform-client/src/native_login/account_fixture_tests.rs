//! Native account fixtures from Platform; every manifest case is classified below.
use super::*;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
enum Suite {
    Oauth,
    Characters,
}
impl Suite {
    fn file(self, name: &str) -> Result<&'static str, Box<dyn std::error::Error>> {
        let files: &[(&str, &str)] = match self {
            Self::Oauth => &[
                (
                    "manifest.json",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/manifest.json"
                    ),
                ),
                (
                    "ticket-invalid-request.json",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/ticket-invalid-request.json"
                    ),
                ),
                (
                    "ticket-request-unsupported.json",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/ticket-request-unsupported.json"
                    ),
                ),
                (
                    "ticket-request.json",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/ticket-request.json"
                    ),
                ),
                (
                    "ticket-response.json",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/ticket-response.json"
                    ),
                ),
                (
                    "ticket-unauthenticated.json",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/ticket-unauthenticated.json"
                    ),
                ),
                (
                    "ticket-unavailable.json",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/ticket-unavailable.json"
                    ),
                ),
                (
                    "token-invalid-client.json",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/token-invalid-client.json"
                    ),
                ),
                (
                    "token-invalid-grant.json",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/token-invalid-grant.json"
                    ),
                ),
                (
                    "token-invalid-request.json",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/token-invalid-request.json"
                    ),
                ),
                (
                    "token-request-code.form",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/token-request-code.form"
                    ),
                ),
                (
                    "token-request-refresh.form",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/token-request-refresh.form"
                    ),
                ),
                (
                    "token-request-unknown-client.form",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/token-request-unknown-client.form"
                    ),
                ),
                (
                    "token-request-unknown-field.form",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/token-request-unknown-field.form"
                    ),
                ),
                (
                    "token-request-unsupported-grant.form",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/token-request-unsupported-grant.form"
                    ),
                ),
                (
                    "token-response.json",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/token-response.json"
                    ),
                ),
                (
                    "token-unsupported-grant.json",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-oauth-ticket-v1/token-unsupported-grant.json"
                    ),
                ),
            ],
            Self::Characters => &[
                (
                    "characters-empty.json",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-characters-v1/characters-empty.json"
                    ),
                ),
                (
                    "characters-response.json",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-characters-v1/characters-response.json"
                    ),
                ),
                (
                    "characters-unauthenticated.json",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-characters-v1/characters-unauthenticated.json"
                    ),
                ),
                (
                    "manifest.json",
                    include_str!(
                        "../../../../docs/contracts/fixtures/platform-native-characters-v1/manifest.json"
                    ),
                ),
            ],
        };
        files
            .iter()
            .find(|(file, _)| *file == name)
            .map(|(_, body)| *body)
            .ok_or_else(|| format!("fixture not included: {name}").into())
    }
    fn manifest(self) -> Result<Manifest, Box<dyn std::error::Error>> {
        Ok(serde_json::from_str(self.file("manifest.json")?)?)
    }
}

#[derive(Deserialize)]
struct Manifest {
    client_id: String,
    synthetic_access_token: String,
    cases: Vec<Case>,
    files: Vec<FixtureFile>,
    #[serde(flatten)]
    metadata: BTreeMap<String, serde_json::Value>,
}
#[derive(Deserialize)]
struct FixtureFile {
    file: String,
    bytes: usize,
    sha256: String,
}
#[derive(Deserialize)]
struct Case {
    #[serde(rename = "case")]
    name: String,
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    request: Option<String>,
    status: u16,
    response_headers: BTreeMap<String, String>,
    response: Option<String>,
    normalized: Vec<String>,
}
impl Case {
    fn response(&self, suite: Suite) -> Result<(u16, String, String), Box<dyn std::error::Error>> {
        let headers = self
            .response_headers
            .iter()
            .map(|(k, v)| format!("{k}: {v}\r\n"))
            .collect::<String>();
        Ok((
            self.status,
            headers,
            self.response
                .as_deref()
                .map(|f| suite.file(f))
                .transpose()?
                .unwrap_or_default()
                .to_owned(),
        ))
    }
    fn bearer(&self, manifest: &Manifest) -> AccessToken {
        let bearer = self
            .headers
            .get("authorization")
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or(&manifest.synthetic_access_token);
        AccessToken(SecretText(bearer.to_owned()))
    }
}
fn metadata<'a>(manifest: &'a Manifest, key: &str) -> Result<&'a str, Box<dyn std::error::Error>> {
    manifest
        .metadata
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("manifest field {key}").into())
}
fn run_fixture(test: fn() -> TestResult) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || test().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "fixture thread panicked")?
        .map_err(Into::into)
}
fn runtime() -> Result<tokio::runtime::Runtime, std::io::Error> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}
fn client(server: &MockServer, client_id: &str) -> Result<NativeLoginClient, PlatformClientError> {
    let config = PlatformClientConfig::new(&server.base)?;
    NativeLoginClient::new(&config, &config, client_id)
}
fn assert_request(server: &MockServer, case: &Case, body: &str, count: usize) -> TestResult {
    let requests = server.requests.lock().map_err(|_| "request lock")?;
    assert_eq!(requests.len(), count, "{}", case.name);
    for request in requests.iter() {
        assert!(
            request.starts_with(&format!("{} {} HTTP/1.1\r\n", case.method, case.path)),
            "{}",
            case.name
        );
        assert_eq!(
            request_body(request).as_bytes(),
            body.as_bytes(),
            "{}",
            case.name
        );
        let headers: BTreeMap<_, _> = request
            .split_once("\r\n\r\n")
            .ok_or("HTTP headers")?
            .0
            .lines()
            .filter_map(|line| {
                line.split_once(':')
                    .map(|(k, v)| (k.to_ascii_lowercase(), v.trim()))
            })
            .collect();
        assert_eq!(
            headers.contains_key("authorization"),
            case.path != "/oauth/token"
        );
        for (key, value) in &case.headers {
            assert_eq!(
                headers.get(key).copied(),
                Some(value.as_str()),
                "{}: {key}",
                case.name
            );
        }
    }
    Ok(())
}
fn assert_redacted(text: &str, manifest: &Manifest) -> TestResult {
    for key in [
        "synthetic_authorization_code",
        "synthetic_refresh_token",
        "code_verifier",
    ] {
        if let Some(secret) = manifest
            .metadata
            .get(key)
            .and_then(serde_json::Value::as_str)
        {
            assert!(!text.contains(secret));
        }
    }
    assert!(!text.contains(&manifest.synthetic_access_token));
    if let Some(placeholders) = manifest
        .metadata
        .get("placeholders")
        .and_then(serde_json::Value::as_object)
    {
        for secret in placeholders.values() {
            assert!(!text.contains(secret.as_str().ok_or("placeholder")?));
        }
    }
    Ok(())
}

#[test]
fn oauth_token_matches_platform_native_oauth_ticket_v1() -> TestResult {
    run_fixture(|| {
        let suite = Suite::Oauth;
        let manifest = suite.manifest()?;
        let runtime = runtime()?;
        for case in manifest.cases.iter().filter(|c| c.path == "/oauth/token") {
            let request_file = case.request.as_deref().ok_or("token request")?;
            if request_file == "token-request-refresh.form" {
                // No refresh-token API exists. Its response is identical to code success.
                assert_eq!(case.response.as_deref(), Some("token-response.json"));
                continue;
            }
            let request = suite.file(request_file)?;
            let form: BTreeMap<_, _> = url::form_urlencoded::parse(request.as_bytes())
                .into_owned()
                .collect();
            let client_id = form.get("client_id").ok_or("client_id")?;
            let server = spawn_fixture_server(&runtime, vec![case.response(suite)?])?;
            let client = client(&server, client_id)?;
            let result = runtime.block_on(client.exchange_code_challenged(
                metadata(&manifest, "synthetic_authorization_code")?,
                metadata(&manifest, "code_verifier")?,
                metadata(&manifest, "redirect_uri")?,
                None,
                CancellationToken::new(),
            ));
            assert_redacted(&format!("{result:?}"), &manifest)?;
            if case.status == 200 {
                let TokenExchange::Token(token) = result? else {
                    return Err("expected token".into());
                };
                let response: serde_json::Value =
                    serde_json::from_str(suite.file(case.response.as_deref().ok_or("response")?)?)?;
                assert_eq!(
                    token.0.expose(),
                    response["access_token"].as_str().ok_or("access token")?
                );
                assert_eq!(case.normalized, ["access_token", "refresh_token"]);
            } else {
                let error = result.err().ok_or("expected refusal")?;
                assert_eq!(error, NativeLoginError::TokenRefused, "{}", case.name);
                assert_redacted(&error.to_string(), &manifest)?;
            }
            // Unsupported grant/extra-field request shapes cannot be produced by the SDK;
            // their RFC error responses above are still exercised through its valid exchange.
            let expected = match request_file {
                "token-request-code.form" | "token-request-unknown-client.form" => {
                    request.to_owned()
                }
                "token-request-unknown-field.form" | "token-request-unsupported-grant.form" => {
                    suite.file("token-request-code.form")?.to_owned()
                }
                _ => return Err(format!("unclassified token request {request_file}").into()),
            };
            assert_request(&server, case, &expected, 1)?;
        }
        Ok(())
    })
}

#[test]
fn ticket_issuance_matches_platform_native_oauth_ticket_v1() -> TestResult {
    run_fixture(|| {
        let suite = Suite::Oauth;
        let manifest = suite.manifest()?;
        let runtime = runtime()?;
        for case in manifest
            .cases
            .iter()
            .filter(|c| c.path == "/api/v1/game-auth/tickets")
        {
            let mut responses = vec![case.response(suite)?];
            if case.status == 429 {
                responses.push(
                    manifest
                        .cases
                        .iter()
                        .find(|c| c.name == "ticket-success")
                        .ok_or("ticket success")?
                        .response(suite)?,
                );
            }
            let server = spawn_fixture_server(&runtime, responses)?;
            let (result, elapsed) =
                issue_with_paused_clock(&runtime, &server, &manifest, case, case.status == 429)?;
            assert_redacted(&format!("{result:?}"), &manifest)?;
            if case.status == 200 || case.status == 429 {
                let ticket = result?;
                let expected: serde_json::Value =
                    serde_json::from_str(suite.file("ticket-response.json")?)?;
                assert_eq!(
                    ticket.secret.expose(),
                    expected["ticket"].as_str().ok_or("ticket")?
                );
                if case.status == 429 {
                    assert_eq!(elapsed, Duration::from_millis(30_001));
                } else {
                    assert_eq!(case.normalized, ["ticket"]);
                }
            } else {
                let error = result.err().ok_or("expected ticket refusal")?;
                assert_eq!(
                    error,
                    NativeLoginError::TicketRefused(if case.status == 401 {
                        PublicClass::AuthenticationRequired
                    } else {
                        PublicClass::TemporarilyUnavailable
                    }),
                    "{}",
                    case.name
                );
                assert_redacted(&error.to_string(), &manifest)?;
            }
            // SDK always authenticates and always sends protocol 1. Invalid-version and
            // missing-bearer request vectors are server-only; test their response handling.
            assert_request(
                &server,
                case,
                suite.file("ticket-request.json")?,
                if case.status == 429 { 2 } else { 1 },
            )?;
        }
        let limited = manifest
            .cases
            .iter()
            .find(|c| c.status == 429)
            .ok_or("rate limit case")?;
        // Headers over the contract limit are refused without a wait or a retry.
        let mut excessive = limited.response(suite)?;
        excessive.1 = excessive.1.replace("retry-after: 30", "retry-after: 31");
        let scenarios = [
            (vec![excessive], 1, Duration::ZERO),
            (
                vec![limited.response(suite)?, limited.response(suite)?],
                2,
                Duration::from_millis(30_001),
            ),
        ];
        for (responses, count, elapsed) in scenarios {
            let server = spawn_fixture_server(&runtime, responses)?;
            let (result, actual_elapsed) =
                issue_with_paused_clock(&runtime, &server, &manifest, limited, count == 2)?;
            assert_eq!(
                result.err(),
                Some(NativeLoginError::TicketRefused(
                    PublicClass::TemporarilyUnavailable
                ))
            );
            assert_request(&server, limited, suite.file("ticket-request.json")?, count)?;
            assert_eq!(actual_elapsed, elapsed);
        }
        // The acceptance criterion also names HTTP 400 (the manifest uses 422).
        let case = manifest
            .cases
            .iter()
            .find(|c| c.name == "ticket-unsupported-version")
            .ok_or("invalid ticket case")?;
        let (_, headers, body) = case.response(suite)?;
        let server = spawn_fixture_server(&runtime, vec![(400, headers, body)])?;
        assert_eq!(
            runtime
                .block_on(
                    client(&server, &manifest.client_id)?
                        .issue_native_ticket(case.bearer(&manifest), CancellationToken::new())
                )
                .err(),
            Some(NativeLoginError::TicketRefused(
                PublicClass::TemporarilyUnavailable
            ))
        );
        Ok(())
    })
}

#[test]
fn native_characters_match_platform_native_characters_v1() -> TestResult {
    run_fixture(|| {
        let suite = Suite::Characters;
        let manifest = suite.manifest()?;
        let runtime = runtime()?;
        for case in &manifest.cases {
            let server = spawn_fixture_server(&runtime, vec![case.response(suite)?])?;
            let token = case.bearer(&manifest);
            let result = runtime.block_on(
                client(&server, &manifest.client_id)?
                    .owner_characters(&token, CancellationToken::new()),
            );
            assert_redacted(&format!("{result:?}"), &manifest)?;
            if case.status == 200 {
                let characters = result?;
                let response: serde_json::Value =
                    serde_json::from_str(suite.file(case.response.as_deref().ok_or("response")?)?)?;
                let expected = response["characters"].as_array().ok_or("characters")?;
                assert_eq!(characters.len(), expected.len());
                for (actual, expected) in characters.iter().zip(expected) {
                    assert_eq!(actual.character_id, expected["character_id"]);
                    assert_eq!(actual.world_id, expected["world_id"]);
                    assert_eq!(actual.name, expected["name"]);
                    assert_eq!(
                        actual.availability,
                        match expected["availability"].as_str() {
                            Some("AVAILABLE") => CharacterAvailability::Available,
                            Some("UNAVAILABLE") => CharacterAvailability::Unavailable,
                            _ => return Err("unknown availability".into()),
                        }
                    );
                }
            } else {
                let error = result.err().ok_or("expected characters refusal")?;
                assert_eq!(
                    error,
                    if case.status == 401 {
                        NativeLoginError::TokenRefused
                    } else {
                        NativeLoginError::Platform(PlatformClientError::HttpStatus(case.status))
                    },
                    "{}",
                    case.name
                );
                assert_redacted(&error.to_string(), &manifest)?;
            }
            assert_request(&server, case, "", 1)?;
        }
        let case = &manifest.cases[0];
        let mut response: serde_json::Value =
            serde_json::from_str(suite.file("characters-response.json")?)?;
        let character = response["characters"][0].clone();
        let id = character["character_id"].as_str().ok_or("character id")?;
        let (prefix, _) = id.rsplit_once('-').ok_or("uuid prefix")?;
        for count in [64, 65] {
            response["characters"] = (1..=count)
                .map(|index| {
                    let mut character = character.clone();
                    character["character_id"] = serde_json::json!(format!("{prefix}-{index:012x}"));
                    character
                })
                .collect();
            let body = serde_json::to_string(&response)?;
            assert!(body.len() <= 16_384);
            let server = spawn_fixture_server(&runtime, vec![(200, String::new(), body)])?;
            let result = runtime.block_on(
                client(&server, &manifest.client_id)?
                    .owner_characters(&case.bearer(&manifest), CancellationToken::new()),
            );
            if count == 64 {
                assert_eq!(result?.len(), 64);
            } else {
                assert_eq!(
                    result.err(),
                    Some(NativeLoginError::Platform(
                        PlatformClientError::InvalidPayload
                    ))
                );
            }
        }
        let mut oversized = suite.file("characters-response.json")?.to_owned();
        oversized.push_str(&" ".repeat(16_385 - oversized.len()));
        let server = spawn_fixture_server(&runtime, vec![(200, String::new(), oversized)])?;
        assert_eq!(
            runtime
                .block_on(
                    client(&server, &manifest.client_id)?
                        .owner_characters(&case.bearer(&manifest), CancellationToken::new())
                )
                .err(),
            Some(NativeLoginError::Platform(
                PlatformClientError::ResponseTooLarge
            ))
        );
        Ok(())
    })
}

#[test]
fn platform_native_manifests_cover_every_case() -> TestResult {
    use sha2::{Digest, Sha256};
    for suite in [Suite::Oauth, Suite::Characters] {
        let manifest = suite.manifest()?;
        for file in &manifest.files {
            let bytes = suite.file(&file.file)?.as_bytes();
            assert_eq!(bytes.len(), file.bytes);
            assert_eq!(
                Sha256::digest(bytes)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
                file.sha256
            );
        }
        assert_eq!(
            manifest.cases.len(),
            match suite {
                Suite::Oauth => 14,
                Suite::Characters => 7,
            }
        );
        for case in &manifest.cases {
            assert!(matches!(
                case.path.as_str(),
                "/oauth/token"
                    | "/api/v1/game-auth/tickets"
                    | "/api/v1/game-auth/native-characters"
            ));
            match (case.path.as_str(), case.request.as_deref()) {
                (
                    "/oauth/token",
                    Some("token-request-code.form" | "token-request-unknown-client.form"),
                )
                | ("/api/v1/game-auth/tickets", Some("ticket-request.json"))
                | ("/api/v1/game-auth/native-characters", None) => {
                    assert_eq!(
                        case.method,
                        if case.path == "/api/v1/game-auth/native-characters" {
                            "GET"
                        } else {
                            "POST"
                        }
                    );
                }
                (
                    "/oauth/token",
                    Some(
                        "token-request-refresh.form"
                        | "token-request-unknown-field.form"
                        | "token-request-unsupported-grant.form",
                    ),
                )
                | ("/api/v1/game-auth/tickets", Some("ticket-request-unsupported.json")) => {
                    // Explicitly classified request-only exclusions; see endpoint tests and report.
                    assert!(case.name == "token-refresh-success" || case.status >= 400);
                }
                _ => return Err(format!("unclassified manifest case {}", case.name).into()),
            }
            if let Some(request) = &case.request {
                suite.file(request)?;
            }
            case.response(suite)?;
            for field in &case.normalized {
                assert!(matches!(
                    field.as_str(),
                    "access_token" | "refresh_token" | "ticket"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn platform_account_mock_responses_match_fixture_headers() -> TestResult {
    run_fixture(|| {
        let runtime = runtime()?;
        for suite in [Suite::Oauth, Suite::Characters] {
            let manifest = suite.manifest()?;
            for case in &manifest.cases {
                let server = spawn_fixture_server(&runtime, vec![case.response(suite)?])?;
                let (status, headers, body) = runtime.block_on(async {
                    let response = reqwest::Client::builder()
                        .no_proxy()
                        .build()?
                        .get(&server.base)
                        .send()
                        .await?;
                    let status = response.status().as_u16();
                    assert_eq!(
                        response.headers().len(),
                        case.response_headers.len(),
                        "{}",
                        case.name
                    );
                    let headers: BTreeMap<String, String> = response
                        .headers()
                        .iter()
                        .map(|(key, value)| {
                            Ok((key.as_str().to_owned(), value.to_str()?.to_owned()))
                        })
                        .collect::<Result<_, reqwest::header::ToStrError>>()?;
                    Ok::<_, Box<dyn std::error::Error>>((status, headers, response.text().await?))
                })?;
                assert_eq!(status, case.status, "{}", case.name);
                assert_eq!(headers, case.response_headers, "{}", case.name);
                assert_eq!(
                    body.as_bytes(),
                    case.response(suite)?.2.as_bytes(),
                    "{}",
                    case.name
                );
            }
        }
        Ok(())
    })
}

#[test]
fn ticket_refuses_missing_or_malformed_retry_after() -> TestResult {
    run_fixture(|| {
        let suite = Suite::Oauth;
        let manifest = suite.manifest()?;
        let limited = manifest
            .cases
            .iter()
            .find(|case| case.name == "ticket-rate-limited")
            .ok_or("rate-limit fixture")?;
        let runtime = runtime()?;
        for value in [
            None,
            Some("abc"),
            Some("-1"),
            Some("1.5"),
            Some("Wed, 21 Oct 2015 07:28:00 GMT"),
        ] {
            let mut headers = limited.response_headers.clone();
            headers.remove("retry-after");
            if let Some(value) = value {
                headers.insert("retry-after".to_owned(), value.to_owned());
            }
            let headers = headers
                .iter()
                .map(|(k, v)| format!("{k}: {v}\r\n"))
                .collect();
            let server = spawn_fixture_server(
                &runtime,
                vec![(limited.status, headers, limited.response(suite)?.2)],
            )?;
            let (result, elapsed) =
                issue_with_paused_clock(&runtime, &server, &manifest, limited, false)?;
            assert_eq!(
                result.err(),
                Some(NativeLoginError::TicketRefused(
                    PublicClass::TemporarilyUnavailable
                )),
                "{value:?}"
            );
            assert_request(&server, limited, suite.file("ticket-request.json")?, 1)?;
            assert_eq!(elapsed, Duration::ZERO);
        }
        Ok(())
    })
}

// Keep I/O progressing without Tokio auto-advancing a pending HTTP deadline.
async fn paused_io<F: std::future::Future>(future: F) -> F::Output {
    tokio::time::pause();
    TICKET_WAIT.with(|clock| clock.set((None, None)));
    let heartbeat = tokio::spawn(async {
        loop {
            tokio::task::yield_now().await;
        }
    });
    let result = future.await;
    heartbeat.abort();
    tokio::time::resume();
    result
}

async fn poll_ticket<T>(
    mut future: std::pin::Pin<&mut impl std::future::Future<Output = T>>,
    stop_at_wait: bool,
) -> Result<Option<T>, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let std::task::Poll::Ready(result) = futures::poll!(future.as_mut()) {
            return Ok(Some(result));
        }
        if stop_at_wait && TICKET_WAIT.with(|clock| clock.get().0.is_some()) {
            return Ok(None);
        }
        if Instant::now() >= deadline {
            return Err("ticket HTTP exchange did not progress".into());
        }
        tokio::task::yield_now().await;
    }
}

fn issue_with_paused_clock(
    runtime: &tokio::runtime::Runtime,
    server: &MockServer,
    manifest: &Manifest,
    case: &Case,
    expect_wait: bool,
) -> Result<(Result<NativeTicket, NativeLoginError>, Duration), Box<dyn std::error::Error>> {
    let client = client(server, &manifest.client_id)?;
    runtime.block_on(paused_io(async {
        let started = tokio::time::Instant::now();
        let mut call =
            Box::pin(client.issue_native_ticket(case.bearer(manifest), CancellationToken::new()));
        let initial = poll_ticket(call.as_mut(), true).await?;
        let result = if expect_wait {
            assert!(initial.is_none(), "ticket never entered retry wait");
            assert_eq!(server.requests.lock().map_err(|_| "request lock")?.len(), 1);
            // Pre-register barriers so completion proves Tokio processed the timer wheel.
            // The 1 ms margin accommodates Tokio's documented timer granularity.
            let early = tokio::time::sleep(Duration::from_millis(29_998));
            let late = tokio::time::sleep(Duration::from_millis(30_000));
            let mut early = Box::pin(early);
            let mut late = Box::pin(late);
            assert!(futures::poll!(early.as_mut()).is_pending());
            assert!(futures::poll!(late.as_mut()).is_pending());
            tokio::time::advance(Duration::from_millis(29_999)).await;
            early.await;
            assert!(futures::poll!(call.as_mut()).is_pending());
            assert!(
                TICKET_WAIT.with(|clock| clock.get().1.is_none()),
                "retry wait ended before 30 seconds"
            );
            assert_eq!(server.requests.lock().map_err(|_| "request lock")?.len(), 1);
            tokio::time::advance(Duration::from_millis(2)).await;
            late.await;
            let result = poll_ticket(call.as_mut(), false)
                .await?
                .ok_or("ticket result")?;
            let (start, end) = TICKET_WAIT.with(|clock| clock.get());
            assert!(
                end.ok_or("sleep did not finish")? - start.ok_or("sleep did not start")?
                    >= Duration::from_secs(30)
            );
            let times = server.request_times.lock().map_err(|_| "timestamps lock")?;
            assert_eq!(times.len(), 2);
            assert!(times[1] - times[0] >= Duration::from_secs(30));
            result
        } else {
            initial.ok_or("ticket unexpectedly entered retry wait")?
        };
        Ok((result, tokio::time::Instant::now() - started))
    }))
}

#[test]
fn ticket_cancellation_interrupts_platform_retry_wait() -> TestResult {
    run_fixture(|| {
        let suite = Suite::Oauth;
        let manifest = suite.manifest()?;
        let case = manifest
            .cases
            .iter()
            .find(|c| c.name == "ticket-rate-limited")
            .ok_or("rate limit fixture")?;
        let success = manifest
            .cases
            .iter()
            .find(|c| c.name == "ticket-success")
            .ok_or("ticket success")?;
        let runtime = runtime()?;
        let server = spawn_fixture_server(
            &runtime,
            vec![case.response(suite)?, success.response(suite)?],
        )?;
        let client = client(&server, &manifest.client_id)?;
        runtime.block_on(paused_io(async {
            let cancellation = CancellationToken::new();
            let mut call =
                Box::pin(client.issue_native_ticket(case.bearer(&manifest), cancellation.clone()));
            assert!(poll_ticket(call.as_mut(), true).await?.is_none());
            tokio::time::advance(Duration::from_secs(5)).await;
            let cancelled_at = tokio::time::Instant::now();
            let wait_started = TICKET_WAIT
                .with(|clock| clock.get().0)
                .ok_or("retry wait did not start")?;
            assert_eq!(cancelled_at - wait_started, Duration::from_secs(5));
            cancellation.cancel();
            let std::task::Poll::Ready(result) = futures::poll!(call.as_mut()) else {
                return Err("cancellation did not promptly interrupt retry wait".into());
            };
            assert_eq!(
                result.err(),
                Some(NativeLoginError::Platform(PlatformClientError::Cancelled))
            );
            assert_eq!(tokio::time::Instant::now(), cancelled_at);
            assert!(TICKET_WAIT.with(|clock| clock.get().1.is_none()));
            assert_request(&server, case, suite.file("ticket-request.json")?, 1)?;
            Ok::<_, Box<dyn std::error::Error>>(())
        }))
    })
}
