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
            let body = error_body(wire, class_text, echo);
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
}

fn spawn_server(
    runtime: &tokio::runtime::Runtime,
    responses: Vec<(u16, &'static str, String)>,
) -> Result<MockServer, Box<dyn std::error::Error>> {
    let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0"))?;
    let base = format!("http://127.0.0.1:{}/", listener.local_addr()?.port());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&requests);
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
            if let Ok(mut log) = recorded.lock() {
                log.push(request);
            }
            let response = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n{extra_headers}\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.shutdown().await;
        }
    });
    Ok(MockServer { base, requests })
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
fn owner_character_read_uses_bearer_and_does_not_issue_ticket() -> TestResult {
    let runtime = tokio::runtime::Runtime::new()?;
    let body = format!(
        r#"{{"protocol_version":2,"characters":[{{"character_id":"{CHARACTER}","world_id":"{WORLD}","name":"Local Walker","availability":"AVAILABLE"}}]}}"#
    );
    let server = spawn_server(&runtime, vec![(200, "", body)])?;
    let client = client_for(&server)?;
    let token = AccessToken(SecretText("owner-token".to_owned()));
    let characters =
        runtime.block_on(client.account_characters(&token, CancellationToken::new()))?;
    assert_eq!(characters.len(), 1);
    assert_eq!(characters[0].name, "Local Walker");
    let requests = server.requests.lock().map_err(|_| "request log poisoned")?;
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("GET /api/v1/game-auth/native-characters "));
    assert!(
        requests[0]
            .to_ascii_lowercase()
            .contains("authorization: bearer owner-token")
    );
    assert!(!format!("{token:?}").contains("owner-token"));
    Ok(())
}

#[test]
fn owner_character_read_rejects_unknown_duplicate_and_invalid_metadata() -> TestResult {
    let runtime = tokio::runtime::Runtime::new()?;
    let character = format!(
        r#"{{"character_id":"{CHARACTER}","world_id":"{WORLD}","name":"Local Walker","availability":"AVAILABLE"}}"#
    );
    for body in [
        format!(r#"{{"protocol_version":2,"characters":[{character},{character}]}}"#),
        format!(r#"{{"protocol_version":1,"characters":[{character}]}}"#),
        format!(r#"{{"protocol_version":2,"characters":[{character}],"extra":true}}"#),
        format!(
            r#"{{"protocol_version":2,"characters":[{}]}}"#,
            character.replace("AVAILABLE", "READY")
        ),
        format!(
            r#"{{"protocol_version":2,"characters":[{}]}}"#,
            character.replace(WORLD, "invalid")
        ),
    ] {
        let server = spawn_server(&runtime, vec![(200, "", body)])?;
        let client = client_for(&server)?;
        let token = AccessToken(SecretText("owner-token".to_owned()));
        assert_eq!(
            runtime.block_on(client.account_characters(&token, CancellationToken::new())),
            Err(NativeLoginError::Platform(
                PlatformClientError::InvalidPayload
            ))
        );
    }
    Ok(())
}

#[test]
fn explicit_account_switch_requests_normal_oauth_login_and_consent() -> TestResult {
    let config = PlatformClientConfig::new("http://127.0.0.1:18584")?;
    let client = NativeLoginClient::new(&config, &config, "oteryn-native")?;
    for (switch, expected) in [(false, "consent"), (true, "login consent")] {
        let url = client.authorization_url_for_login(
            "http://127.0.0.1:12345/callback",
            "state",
            "challenge",
            switch,
        )?;
        let pairs = url.query_pairs().collect::<Vec<_>>();
        assert_eq!(pairs.iter().filter(|(name, _)| name == "prompt").count(), 1);
        assert!(
            pairs
                .iter()
                .any(|(name, value)| name == "prompt" && value == expected)
        );
        assert!(
            pairs
                .iter()
                .any(|(name, value)| name == "scope" && value == "game:ticket")
        );
        assert!(
            pairs
                .iter()
                .any(|(name, value)| name == "code_challenge_method" && value == "S256")
        );
    }
    Ok(())
}
