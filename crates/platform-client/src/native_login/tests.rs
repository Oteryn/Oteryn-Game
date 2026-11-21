#[path = "account_fixture_tests.rs"]
mod account_fixtures;
#[path = "bazaar_tests.rs"]
mod bazaar;
#[path = "challenge_tests.rs"]
mod challenge;
#[path = "channel_probe_fixture_tests.rs"]
mod channel_probe_fixtures;
#[path = "directory_fixture_tests.rs"]
mod directory_fixtures;
#[path = "platform_fixture_tests.rs"]
mod platform_fixtures;

use super::*;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ATTEMPT: &str = "01890a5d-ac96-774b-bcce-b302099a8057";
const CHARACTER: &str = "01890a5d-ac96-774b-bcce-b302099a8058";
const WORLD: &str = "01890a5d-ac96-774b-bcce-b302099a8059";
const CHANNEL: &str = "01890a5d-ac96-774b-bcce-b302099a805a";
const JWS: &str = "eyJhbGciOiJFZERTQSJ9.eyJzdWIiOiJ4In0.c2lnbmF0dXJl";

fn attempt() -> AttemptRef {
    AttemptRef(ATTEMPT.to_owned())
}

fn success_body(attempt_ref: &str) -> String {
    format!(
        r#"{{"protocol_version":2,"attempt_ref":"{attempt_ref}","world_id":"{WORLD}","channel_id":"{CHANNEL}","endpoint":{{"host":"game.example.test","port":7171,"tls_server_name":"game.example.test","alpn":"oteryn-game/1","protocol_major":1,"transport_profile":1}},"grant":{{"profile":"oteryn-pre-admission-v1","token":"{JWS}","valid_for_seconds":30}}}}"#
    )
}

fn error_body(code: &str, class: &str, attempt_ref: Option<&str>) -> String {
    let echo = attempt_ref.map_or_else(|| "null".to_owned(), |value| format!("\"{value}\""));
    format!(
        r#"{{"protocol_version":2,"error":{{"code":"{code}","public_class":"{class}"}},"attempt_ref":{echo}}}"#
    )
}

#[test]
fn attempt_ref_is_canonical_uuid_v7() -> TestResult {
    let first = AttemptRef::generate()?;
    let second = AttemptRef::generate()?;
    assert!(parse_uuid_v7(first.as_str()).is_some());
    assert_ne!(first, second);
    assert_eq!(first.as_str(), first.as_str().to_ascii_lowercase());
    Ok(())
}

#[test]
fn uuid_v7_parser_rejects_other_shapes() {
    assert!(parse_uuid_v7(ATTEMPT).is_some());
    for rejected in [
        "01890A5D-AC96-774B-BCCE-B302099A8057",
        "01890a5d-ac96-474b-bcce-b302099a8057",
        "01890a5d-ac96-774b-7cce-b302099a8057",
        "01890a5dac96774bbcceb302099a8057",
        "01890a5d-ac96-774b-bcce-b302099a805",
    ] {
        assert!(parse_uuid_v7(rejected).is_none(), "{rejected}");
    }
}

#[test]
fn success_is_strictly_decoded() -> TestResult {
    let grant = parse_login_success(success_body(ATTEMPT).as_bytes(), &attempt(), Instant::now())?;
    assert_eq!(grant.endpoint.port, 7171);
    assert_eq!(grant.endpoint.tls_server_name, "game.example.test");
    assert_eq!(grant.world_id, WORLD);
    assert_eq!(grant.grant_bytes(), JWS.as_bytes());
    assert!(!format!("{grant:?}").contains(JWS));
    Ok(())
}

#[test]
fn success_rejects_unknown_missing_and_mismatched_members() {
    let valid = success_body(ATTEMPT);
    let cases = [
        valid.replacen("{", r#"{"extra":1,"#, 1),
        valid.replace(r#""channel_id":"#, r#""channel_id":null,"x":"#),
        valid.replace(r#""tls_server_name":"game.example.test","#, ""),
        valid.replace(r#""alpn":"oteryn-game/1""#, r#""alpn":"h2""#),
        valid.replace(r#""port":7171"#, r#""port":0"#),
        valid.replace(r#""valid_for_seconds":30"#, r#""valid_for_seconds":0"#),
        valid.replace(r#""protocol_version":2"#, r#""protocol_version":1"#),
        valid.replace(JWS, "not-a-jws"),
        valid.replace(r#""host":"game.example.test""#, r#""host":"bad host""#),
        success_body("01890a5d-ac96-774b-bcce-b302099a8000"),
        valid.replacen(r#""world_id""#, r#""world_id":"x","world_id""#, 1),
    ];
    for case in cases {
        assert_eq!(
            parse_login_success(case.as_bytes(), &attempt(), Instant::now()).map(|_| ()),
            Err(PlatformClientError::InvalidPayload),
            "{case}"
        );
    }
}

#[test]
fn every_contract_error_row_decodes() -> TestResult {
    for (wire, code, status, progression, class) in GATEWAY_ERROR_ROWS {
        let class_text = match class {
            PublicClass::RetryLogin => "RETRY_LOGIN",
            PublicClass::AuthenticationRequired => "AUTHENTICATION_REQUIRED",
            PublicClass::TemporarilyUnavailable => "TEMPORARILY_UNAVAILABLE",
            PublicClass::SessionUnavailable => "SESSION_UNAVAILABLE",
            PublicClass::ClientUpdateRequired => "CLIENT_UPDATE_REQUIRED",
            PublicClass::CharacterAlreadyActive => "CHARACTER_ALREADY_ACTIVE",
        };
        for echo in [Some(ATTEMPT), None] {
            let mut body = error_body(wire, class_text, echo);
            if code == GatewayErrorCode::ClientUpdateRequired {
                body.pop();
                body = format!("{body},{UPDATE_POINTER}}}");
            }
            assert_eq!(
                parse_login_error(status, body.as_bytes(), &attempt())?,
                code
            );
        }
        assert_eq!(code.progression(), progression);
        assert_eq!(code.public_class(), class);
    }
    Ok(())
}

#[test]
fn inconsistent_or_raw_security_errors_fail_closed() {
    let ours = attempt();
    for (status, body) in [
        (
            401,
            error_body("NATIVE_LOGIN_TICKET_REJECTED", "AUTHENTICATION_REQUIRED", None),
        ),
        (
            400,
            error_body("NATIVE_LOGIN_RATE_LIMITED", "TEMPORARILY_UNAVAILABLE", None),
        ),
        (
            429,
            error_body("NATIVE_LOGIN_RATE_LIMITED", "RETRY_LOGIN", None),
        ),
        (
            503,
            error_body(
                "NATIVE_LOGIN_UNAVAILABLE",
                "TEMPORARILY_UNAVAILABLE",
                Some("01890a5d-ac96-774b-bcce-b302099a8000"),
            ),
        ),
        (
            503,
            r#"{"protocol_version":2,"error":{"code":"NATIVE_LOGIN_UNAVAILABLE","public_class":"TEMPORARILY_UNAVAILABLE"}}"#.to_owned(),
        ),
    ] {
        assert_eq!(
            parse_login_error(status, body.as_bytes(), &ours),
            Err(PlatformClientError::InvalidPayload),
            "{body}"
        );
    }
}

#[test]
fn secrets_are_redacted() {
    let secret = SecretText("ticket-value".to_owned());
    assert!(!format!("{secret:?}{secret}").contains("ticket-value"));
}

/// A scripted HTTP/1.1 server: one response per connection, request bodies recorded.
struct MockServer {
    base: String,
    requests: Arc<Mutex<Vec<String>>>,
    request_times: Arc<Mutex<Vec<tokio::time::Instant>>>,
}

fn spawn_server(
    runtime: &tokio::runtime::Runtime,
    responses: Vec<(u16, &'static str, String)>,
) -> Result<MockServer, Box<dyn std::error::Error>> {
    spawn_server_with_headers(
        runtime,
        responses
            .into_iter()
            .map(|(status, headers, body)| (status, headers.to_owned(), body))
            .collect(),
    )
}

fn spawn_server_with_headers(
    runtime: &tokio::runtime::Runtime,
    responses: Vec<(u16, String, String)>,
) -> Result<MockServer, Box<dyn std::error::Error>> {
    let responses = responses.into_iter().map(|(status, headers, body)| {
        (status, format!("content-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n{headers}", body.len()), body)
    }).collect();
    spawn_fixture_server(runtime, responses)
}

// Fixture responses are close-delimited: shutdown supplies framing, without
// injecting Content-Type, Content-Length, Connection, or any other header.
fn spawn_fixture_server(
    runtime: &tokio::runtime::Runtime,
    responses: Vec<(u16, String, String)>,
) -> Result<MockServer, Box<dyn std::error::Error>> {
    let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0"))?;
    let base = format!("http://127.0.0.1:{}/", listener.local_addr()?.port());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&requests);
    let request_times = Arc::new(Mutex::new(Vec::new()));
    let times = Arc::clone(&request_times);
    runtime.spawn(async move {
        for (status, extra_headers, body) in responses {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            let mut buffer = Vec::new();
            let mut chunk = [0_u8; 4096];
            let request = loop {
                let Ok(read) = stream.read(&mut chunk).await else {
                    return;
                };
                if read == 0 {
                    return;
                }
                buffer.extend_from_slice(&chunk[..read]);
                let text = String::from_utf8_lossy(&buffer).into_owned();
                if let Some((head, body)) = text.split_once("\r\n\r\n") {
                    let length = head
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                    if body.len() >= length {
                        break text;
                    }
                }
            };
            if let Ok(mut log) = times.lock() {
                log.push(tokio::time::Instant::now());
            }
            if let Ok(mut log) = recorded.lock() {
                log.push(request);
            }
            let response = format!("HTTP/1.1 {status} X\r\n{extra_headers}\r\n{body}");
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.shutdown().await;
        }
    });
    Ok(MockServer {
        base,
        requests,
        request_times,
    })
}

fn client_for(server: &MockServer) -> Result<NativeLoginClient, PlatformClientError> {
    let config = PlatformClientConfig::new(&server.base)?;
    Ok(
        NativeLoginClient::new(&config, &config, "oteryn-native")?.with_pacing(RetryPacing {
            unit: Duration::from_millis(5),
            max_retry_after: Duration::from_secs(2),
        }),
    )
}

fn ticket() -> NativeTicket {
    NativeTicket {
        secret: SecretText("ticket-1".to_owned()),
        usable_until: Instant::now() + MAX_TICKET_LIFETIME,
        requests_sent: AtomicU32::new(0),
    }
}

fn request_body(request: &str) -> &str {
    request.split_once("\r\n\r\n").map_or("", |(_, body)| body)
}

#[test]
fn retryable_rows_reuse_the_identical_request() -> TestResult {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let server = spawn_server(
        &runtime,
        vec![
            (
                503,
                "",
                error_body(
                    "ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED",
                    "TEMPORARILY_UNAVAILABLE",
                    Some(ATTEMPT),
                ),
            ),
            (
                429,
                "retry-after: 0\r\n",
                error_body("NATIVE_LOGIN_RATE_LIMITED", "TEMPORARILY_UNAVAILABLE", None),
            ),
            (200, "", success_body(ATTEMPT)),
        ],
    )?;
    let client = client_for(&server)?;
    let attempt_ref = attempt();
    let login = GatewayLogin {
        attempt_ref: &attempt_ref,
        character_id: CHARACTER,
        channel_id: None,
        client_build: "dev-1",
    };
    let grant =
        runtime.block_on(client.gateway_login(&ticket(), &login, CancellationToken::new()))?;
    assert_eq!(grant.grant_bytes(), JWS.as_bytes());
    let requests = server.requests.lock().map_err(|_| "poisoned")?.clone();
    assert_eq!(requests.len(), 3);
    let first = request_body(&requests[0]);
    assert!(
        requests
            .iter()
            .all(|request| request_body(request) == first)
    );
    assert!(requests[0].starts_with("POST /v1/login HTTP/1.1"));
    let sent: serde_json::Value = serde_json::from_str(first)?;
    assert_eq!(
        sent,
        serde_json::json!({
            "protocol_version": 2,
            "game_login_ticket": "ticket-1",
            "attempt_ref": ATTEMPT,
            "character_id": CHARACTER,
            "channel_id": null,
            "offer": {
                "client_build": "dev-1",
                "client_platform": "windows",
                "transports": [{"protocol_major": 1, "transport_profile": 1, "alpn": "oteryn-game/1"}],
            },
        })
    );
    Ok(())
}

#[test]
fn terminal_rows_stop_and_retries_are_bounded() -> TestResult {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let unavailable = || {
        (
            503,
            "",
            error_body("NATIVE_LOGIN_UNAVAILABLE", "TEMPORARILY_UNAVAILABLE", None),
        )
    };
    let server = spawn_server(&runtime, (0..8).map(|_| unavailable()).collect())?;
    let client = client_for(&server)?.with_pacing(RetryPacing {
        unit: Duration::from_millis(1),
        max_retry_after: Duration::from_secs(1),
    });
    let attempt_ref = attempt();
    let login = GatewayLogin {
        attempt_ref: &attempt_ref,
        character_id: CHARACTER,
        channel_id: None,
        client_build: "dev-1",
    };
    let spent_ticket = ticket();
    let result =
        runtime.block_on(client.gateway_login(&spent_ticket, &login, CancellationToken::new()));
    assert_eq!(
        result.map(|_| ()),
        Err(NativeLoginError::Gateway(GatewayErrorCode::Unavailable))
    );
    assert_eq!(
        server.requests.lock().map_err(|_| "poisoned")?.len(),
        MAX_LOGIN_REQUESTS as usize
    );
    // A reconciliation re-login with the same ticket shares the spent budget.
    let result =
        runtime.block_on(client.gateway_login(&spent_ticket, &login, CancellationToken::new()));
    assert_eq!(result.map(|_| ()), Err(NativeLoginError::TicketSpent));
    assert_eq!(
        server.requests.lock().map_err(|_| "poisoned")?.len(),
        MAX_LOGIN_REQUESTS as usize
    );

    // An expired ticket is never sent.
    let server = spawn_server(&runtime, vec![(200, "", success_body(ATTEMPT))])?;
    let expired = NativeTicket {
        secret: SecretText("ticket-1".to_owned()),
        usable_until: Instant::now(),
        requests_sent: AtomicU32::new(0),
    };
    let result = runtime.block_on(client_for(&server)?.gateway_login(
        &expired,
        &login,
        CancellationToken::new(),
    ));
    assert_eq!(result.map(|_| ()), Err(NativeLoginError::TicketSpent));
    assert!(server.requests.lock().map_err(|_| "poisoned")?.is_empty());

    let server = spawn_server(
        &runtime,
        vec![(
            401,
            "",
            error_body(
                "NATIVE_LOGIN_AUTHENTICATION_REQUIRED",
                "AUTHENTICATION_REQUIRED",
                None,
            ),
        )],
    )?;
    let result = runtime.block_on(client_for(&server)?.gateway_login(
        &ticket(),
        &login,
        CancellationToken::new(),
    ));
    let error = result.map(|_| ()).err();
    assert_eq!(
        error,
        Some(NativeLoginError::Gateway(
            GatewayErrorCode::AuthenticationRequired
        ))
    );
    assert_eq!(
        error.map(NativeLoginError::public_class),
        Some(PublicClass::AuthenticationRequired)
    );
    Ok(())
}

#[test]
fn token_exchange_and_ticket_issuance() -> TestResult {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let server = spawn_server(
        &runtime,
        vec![
            (
                200,
                "",
                r#"{"token_type":"Bearer","access_token":"access-1","expires_in":300,"scope":"game:ticket"}"#.to_owned(),
            ),
            (
                200,
                "",
                r#"{"protocol_version":1,"ticket":"ticket-1","expires_in":120}"#.to_owned(),
            ),
        ],
    )?;
    let client = client_for(&server)?;
    let url = client.authorization_url("http://127.0.0.1:1/callback", "state-1", "challenge")?;
    let query = url.query().unwrap_or_default();
    for expected in [
        "response_type=code",
        "scope=game%3Aticket",
        "code_challenge_method=S256",
        "state=state-1",
        "client_id=oteryn-native",
    ] {
        assert!(query.contains(expected), "{query}");
    }
    let before = Instant::now();
    let issued = runtime.block_on(async {
        let token = client
            .exchange_code(
                "code-1",
                "verifier-1",
                "http://127.0.0.1:1/callback",
                CancellationToken::new(),
            )
            .await?;
        client
            .issue_native_ticket(token, CancellationToken::new())
            .await
    })?;
    assert!(issued.usable_until() <= Instant::now() + MAX_TICKET_LIFETIME);
    assert!(issued.usable_until() > before);
    let requests = server.requests.lock().map_err(|_| "poisoned")?.clone();
    assert!(requests[0].starts_with("POST /oauth/token "));
    assert!(request_body(&requests[0]).contains("code_verifier=verifier-1"));
    assert!(requests[1].starts_with("POST /api/v1/game-auth/tickets "));
    assert!(
        requests[1]
            .to_ascii_lowercase()
            .contains("authorization: bearer access-1")
    );
    Ok(())
}

#[test]
fn owner_characters_borrow_token_and_classify_responses() -> TestResult {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let valid = format!(
        r#"{{"protocol_version":2,"characters":[{{"character_id":"{CHARACTER}","world_id":"{WORLD}","name":"Ari","availability":"AVAILABLE"}}]}}"#
    );
    let server = spawn_server(
        &runtime,
        vec![
            (200, "", valid),
            (401, "", String::new()),
            (429, "retry-after: 60\r\n", String::new()),
            (503, "", String::new()),
            (200, "", "x".repeat(16_385)),
            (200, "", r#"{"protocol_version":2,"characters":[]}"#.into()),
            (
                200,
                "",
                r#"{"protocol_version":1,"ticket":"ticket-1","expires_in":60}"#.into(),
            ),
        ],
    )?;
    let client = client_for(&server)?;
    let token = AccessToken(SecretText("owner-access".into()));
    let rows = runtime.block_on(client.owner_characters(&token, CancellationToken::new()))?;
    assert_eq!(rows[0].character_id, CHARACTER);
    assert_eq!(rows[0].availability, CharacterAvailability::Available);
    for expected in [
        NativeLoginError::TokenRefused,
        NativeLoginError::Platform(PlatformClientError::HttpStatus(429)),
        NativeLoginError::Platform(PlatformClientError::HttpStatus(503)),
        NativeLoginError::Platform(PlatformClientError::ResponseTooLarge),
    ] {
        assert_eq!(
            runtime
                .block_on(client.owner_characters(&token, CancellationToken::new()))
                .err(),
            Some(expected)
        );
    }
    assert!(
        runtime
            .block_on(client.owner_characters(&token, CancellationToken::new()))?
            .is_empty()
    );
    runtime.block_on(client.issue_native_ticket(token, CancellationToken::new()))?;
    let requests = server.requests.lock().map_err(|_| "request lock")?;
    assert_eq!(requests.len(), 7);
    for request in &requests[..6] {
        assert!(request.starts_with("GET /api/v1/game-auth/native-characters "));
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer owner-access")
        );
    }
    assert!(requests[6].starts_with("POST /api/v1/game-auth/tickets "));
    Ok(())
}

#[path = "coin_purchase_tests.rs"]
mod coin_purchase_tests;

// Observe entry/exit of the real Tokio sleep; never replace its duration.
thread_local! {
    static TICKET_WAIT: std::cell::Cell<(Option<tokio::time::Instant>, Option<tokio::time::Instant>)> = const {
        std::cell::Cell::new((None, None))
    };
}
pub(super) fn ticket_wait(finished: bool) {
    TICKET_WAIT.with(|clock| {
        let (start, end) = clock.get();
        let now = Some(tokio::time::Instant::now());
        clock.set(if finished { (start, now) } else { (now, end) });
    });
}

// Thread-local controlled time affects only opt-in current-thread fixture tests.
// Other tests and every production build retain real sleeps.
thread_local! {
    static CLOCK: std::cell::RefCell<Option<(Instant, Duration)>> = const {
        std::cell::RefCell::new(None)
    };
}

fn pause_clock() {
    let received = Instant::now();
    CLOCK.with(|clock| *clock.borrow_mut() = Some((received, Duration::ZERO)));
}

fn clock_now() -> Result<Instant, Box<dyn std::error::Error>> {
    CLOCK.with(|clock| {
        clock
            .borrow()
            .as_ref()
            .map(|(base, elapsed)| *base + *elapsed)
            .ok_or_else(|| "fixture clock is not paused".into())
    })
}

fn clock_elapsed() -> Duration {
    CLOCK.with(|clock| {
        clock
            .borrow()
            .as_ref()
            .map_or(Duration::ZERO, |(_, elapsed)| *elapsed)
    })
}

pub(super) fn delay(duration: Duration) -> Duration {
    CLOCK.with(|clock| match clock.borrow_mut().as_mut() {
        Some((_, elapsed)) => {
            *elapsed += duration;
            Duration::ZERO
        }
        None => duration,
    })
}

const UPDATE_POINTER: &str = r#""update":{"update_channel":"stable","minimum_release_sequence":null,"current_release_sequence":null,"manual_download_url":"https://oteryn.com/download"}"#;

fn update_refusal(pointer: &str) -> String {
    format!(
        "{{{},{pointer}}}",
        error_body(
            "NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED",
            "CLIENT_UPDATE_REQUIRED",
            None,
        )
        .trim_matches(['{', '}'])
    )
}

#[test]
fn client_build_gate_update_requires_a_valid_pointer() {
    let valid = update_refusal(UPDATE_POINTER);
    assert!(parse_login_error(409, valid.as_bytes(), &attempt()).is_ok());
    let protocol_3 = valid.replace(r#""protocol_version":2"#, r#""protocol_version":3"#);
    assert_eq!(
        parse_gateway_refusal(3, 409, protocol_3.as_bytes(), &attempt())
            .map(|refusal| refusal.code()),
        Ok(GatewayErrorCode::ClientUpdateRequired),
    );
    assert!(parse_gateway_refusal(2, 409, protocol_3.as_bytes(), &attempt()).is_err());
    assert!(parse_login_error(503, valid.as_bytes(), &attempt()).is_err());
    assert!(
        parse_login_error(
            503,
            valid
                .replace(
                    "NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED",
                    "NATIVE_LOGIN_MAINTENANCE"
                )
                .replace(
                    r#""public_class":"CLIENT_UPDATE_REQUIRED""#,
                    r#""public_class":"TEMPORARILY_UNAVAILABLE""#
                )
                .as_bytes(),
            &attempt()
        )
        .is_err()
    );
    for invalid in [
        error_body(
            "NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED",
            "CLIENT_UPDATE_REQUIRED",
            None,
        ),
        update_refusal(r#""update":null"#),
        valid.replace("https://", "http://"),
        valid.replace("https://", "file://"),
        valid.replace("https://", "https:///"),
        valid.replace("https://", "https://user@"),
        valid.replace(
            r#""update_channel":"stable""#,
            r#""update_channel":"stable","update_channel":"beta""#,
        ),
        valid.replace(r#""minimum_release_sequence":null,"#, ""),
        valid.replace(
            r#""current_release_sequence":null"#,
            r#""current_release_sequence":0"#,
        ),
        valid.replace(
            r#""current_release_sequence":null"#,
            r#""current_release_sequence":9007199254740992"#,
        ),
        valid.replace(
            r#""update_channel":"stable""#,
            r#""update_channel":"other""#,
        ),
        valid.replace(r#""update":{"#, r#""update":{"extra":0,"#),
    ] {
        assert!(
            parse_login_error(409, invalid.as_bytes(), &attempt()).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn client_build_gate_maintenance_requires_a_fresh_sign_in() -> TestResult {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let refusal = error_body(
        "NATIVE_LOGIN_MAINTENANCE",
        "TEMPORARILY_UNAVAILABLE",
        Some(ATTEMPT),
    );
    let server = spawn_server(&runtime, vec![(503, "Retry-After: 2\r\n", refusal)])?;
    let attempt_ref = attempt();
    let client = client_for(&server)?;
    let login = GatewayLogin {
        attempt_ref: &attempt_ref,
        character_id: CHARACTER,
        channel_id: None,
        client_build: "dev-1",
    };
    let started = Instant::now();
    let result = runtime.block_on(client.with_release(Some(test_release()?)).gateway_login(
        &ticket(),
        &login,
        CancellationToken::new(),
    ));
    let Err(NativeLoginError::Maintenance { retry_at }) = result else {
        return Err("expected maintenance with a fresh sign-in deadline".into());
    };
    assert!(retry_at >= started + Duration::from_secs(2));
    assert_eq!(server.requests.lock().map_err(|_| "poisoned")?.len(), 1);
    assert_eq!(
        GatewayErrorCode::Maintenance.progression(),
        Progression::Retryable
    );
    Ok(())
}

fn test_release() -> Result<ReleaseIdentity, PlatformClientError> {
    ReleaseIdentity::new(UpdateChannel::Stable, "rel-42", 42)
        .ok_or(PlatformClientError::InvalidPayload)
}

#[test]
fn client_build_gate_release_is_in_the_ordered_offer() -> TestResult {
    const LEGACY: &str = r#"{"client_build":"dev-1","client_platform":"windows","transports":[{"protocol_major":1,"transport_profile":1,"alpn":"oteryn-game/1"}]}"#;
    const WITH_RELEASE: &str = r#"{"client_build":"dev-1","client_platform":"windows","release":{"update_channel":"stable","release_id":"rel-42","release_sequence":42},"transports":[{"protocol_major":1,"transport_profile":1,"alpn":"oteryn-game/1"}]}"#;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let server = spawn_server(&runtime, vec![])?;
    let attempt_ref = attempt();
    let login = GatewayLogin {
        attempt_ref: &attempt_ref,
        character_id: CHARACTER,
        channel_id: None,
        client_build: "dev-1",
    };
    let handle = format!("otr1.{}", "A".repeat(43));
    let sig = "A".repeat(86);
    let proof = RecoveryDeviceProof {
        kid: &handle[5..],
        sig: &sig,
    };
    for (release, offer, digest) in [
        (
            None,
            LEGACY,
            "a37bedd3e125c473195a8a3789f42bbe2a8cd599540527c7af1b894783f251b2",
        ),
        (
            Some(test_release()?),
            WITH_RELEASE,
            "95aca8c3f9bb87fe3acfe499acb2b8d4e66e1a20ef935f217f90c3b4f22f30da",
        ),
    ] {
        let client = client_for(&server)?.with_release(release);
        for version in [2, 3] {
            assert_eq!(
                std::str::from_utf8(&client.login_request(&ticket(), &login, version)?)?,
                format!(
                    r#"{{"protocol_version":{version},"game_login_ticket":"ticket-1","attempt_ref":"{ATTEMPT}","character_id":"{CHARACTER}","channel_id":null,"offer":{offer}}}"#
                )
            );
        }
        let proof_suffix = format!(r#","device_proof":{{"kid":"{}","sig":"{sig}"}}"#, proof.kid);
        for (device_proof, suffix) in [(None, ""), (Some(&proof), proof_suffix.as_str())] {
            assert_eq!(
                std::str::from_utf8(&client.recovery_request(
                    &handle,
                    &attempt_ref,
                    CHARACTER,
                    "dev-1",
                    device_proof
                )?)?,
                format!(
                    r#"{{"protocol_version":3,"recovery_handle":"{handle}","attempt_ref":"{ATTEMPT}","character_id":"{CHARACTER}","offer":{offer}{suffix}}}"#
                )
            );
        }
        assert_eq!(client.offer_digest("dev-1")?, digest);
    }
    Ok(())
}

#[test]
fn client_build_gate_validates_release_identity_bounds() {
    use crate::updates::MAX_RELEASE_SEQUENCE;
    assert!(ReleaseIdentity::new(UpdateChannel::Beta, "rel-42._", MAX_RELEASE_SEQUENCE).is_some());
    for id in [
        "",
        "dev+1",
        "dev/1",
        "rél",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ] {
        assert!(ReleaseIdentity::new(UpdateChannel::Stable, id, 1).is_none());
    }
    for sequence in [0, MAX_RELEASE_SEQUENCE + 1] {
        assert!(ReleaseIdentity::new(UpdateChannel::Stable, "rel-42", sequence).is_none());
    }
}

#[test]
fn client_build_gate_update_refusal_is_terminal() -> TestResult {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let server = spawn_server(
        &runtime,
        vec![
            (409, "Retry-After: 1\r\n", update_refusal(UPDATE_POINTER)),
            (200, "", success_body(ATTEMPT)),
        ],
    )?;
    let attempt_ref = attempt();
    let client = client_for(&server)?.with_release(Some(test_release()?));
    let login = GatewayLogin {
        attempt_ref: &attempt_ref,
        character_id: CHARACTER,
        channel_id: None,
        client_build: "dev-1",
    };
    let result =
        runtime.block_on(client.gateway_login(&ticket(), &login, CancellationToken::new()));
    let Err(NativeLoginError::UpdateRequired(pointer)) = result else {
        return Err("expected update pointer".into());
    };
    assert_eq!(pointer.minimum_release_sequence, None);
    assert_eq!(pointer.current_release_sequence, None);
    assert_eq!(pointer.manual_download_url, "https://oteryn.com/download");
    assert_eq!(
        GatewayErrorCode::ClientUpdateRequired.progression(),
        Progression::Terminal
    );
    assert_eq!(server.requests.lock().map_err(|_| "poisoned")?.len(), 1);
    Ok(())
}

#[test]
fn client_build_gate_maintenance_retry_after_supports_both_forms_and_bounds() {
    let now = UNIX_EPOCH + Duration::from_secs(1_770_000_000);
    let date = httpdate::fmt_http_date(now + Duration::from_secs(9));
    let past = httpdate::fmt_http_date(now - Duration::from_secs(3));
    for (header, expected) in [
        (Some("9"), 9),
        (Some(date.as_str()), 9),
        (Some(past.as_str()), 1),
        (Some("99999999999999999999999"), u64::MAX),
        (Some("0"), 1),
        (Some("-1"), 1),
        (Some("invalid"), 1),
        (None, 1),
    ] {
        assert_eq!(
            maintenance_delay(header, now),
            Duration::from_secs(expected)
        );
    }
}

#[test]
fn client_build_gate_maintenance_keeps_long_retry_after() {
    let now = UNIX_EPOCH + Duration::from_secs(1_770_000_000);
    let wait = Duration::from_secs(120);
    let date = httpdate::fmt_http_date(now + wait);
    for header in ["120", date.as_str()] {
        assert_eq!(maintenance_delay(Some(header), now), wait);
    }
}

#[test]
fn maintenance_deadline_saturates_without_a_short_retry_or_overflow() {
    let received = Instant::now();
    let deadline =
        maintenance_deadline(received, Some("99999999999999999999999"), SystemTime::now());
    assert!(deadline > received + Duration::from_secs(120));
    let remaining = deadline.duration_since(received).as_secs();
    assert!(
        remaining == u64::MAX
            || received
                .checked_add(Duration::from_secs(remaining + 1))
                .is_none()
    );
}

#[test]
fn client_build_gate_refusal_requires_objects_in_all_positions() {
    for body in [
        r#"[2,{"code":"NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED","public_class":"CLIENT_UPDATE_REQUIRED"},null,{"update_channel":"stable","minimum_release_sequence":null,"current_release_sequence":null,"manual_download_url":"https://oteryn.com/download"}]"#,
        r#"{"protocol_version":2,"error":["NATIVE_LOGIN_MAINTENANCE","TEMPORARILY_UNAVAILABLE"],"attempt_ref":null}"#,
        &update_refusal(r#""update":["stable",null,null,"https://oteryn.com/download"]"#),
    ] {
        for status in [409, 503] {
            assert!(
                parse_login_error(status, body.as_bytes(), &attempt()).is_err(),
                "{body}"
            );
        }
    }
    let valid = update_refusal(UPDATE_POINTER);
    assert!(matches!(
        parse_gateway_refusal(2, 409, valid.as_bytes(), &attempt()),
        Ok(GatewayRefusal::UpdateRequired(_))
    ));
}

#[test]
fn owner_character_feed_rejects_invalid_names_and_duplicate_identities() -> TestResult {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(|e| e.to_string())?;
        let row = serde_json::json!({"character_id": CHARACTER, "world_id": WORLD, "name": "Ari", "availability": "AVAILABLE"});
        let mut feeds = Vec::new();
        for name in ["x".repeat(97), "A\nri".into(), "".into()] {
            let mut invalid = row.clone();
            invalid["name"] = name.into();
            feeds.push(serde_json::json!({"protocol_version": 2, "characters": [invalid]}));
        }
        let mut conflicting = row.clone();
        conflicting["world_id"] = CHANNEL.into();
        feeds.push(serde_json::json!({"protocol_version": 2, "characters": [row.clone(), conflicting]}));
        feeds.push(serde_json::json!({"protocol_version": 2, "characters": [row.clone(), row]}));
        let server = spawn_server(&runtime, feeds.iter().map(|v| (200, "", v.to_string())).collect()).map_err(|e| e.to_string())?;
        let client = client_for(&server).map_err(|e| e.to_string())?;
        let token = AccessToken(SecretText("owner-access".into()));
        for _ in feeds {
            assert_eq!(runtime.block_on(client.owner_characters(&token, CancellationToken::new())).err(), Some(NativeLoginError::Platform(PlatformClientError::InvalidPayload)));
        }
        Ok::<(), String>(())
    })?.join().map_err(|_| "character test panicked")?.map_err(Into::into)
}
