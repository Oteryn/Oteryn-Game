//! Platform-owned gateway vectors, exercised through the production HTTP response path.
use super::*;

const REQUEST: &str =
    include_str!("../../../../docs/contracts/fixtures/platform-native-login-v1/request.json");
const SUCCESS: &str =
    include_str!("../../../../docs/contracts/fixtures/platform-native-login-v1/success.json");
const ERRORS: &str =
    include_str!("../../../../docs/contracts/fixtures/platform-native-login-v1/errors.json");
const SEQUENCES: &str =
    include_str!("../../../../docs/contracts/fixtures/platform-native-login-v1/sequences.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Vector {
    status: u16,
    body: String,
    progression: Option<String>,
    retry_after: Option<String>,
}
impl Vector {
    fn response(&self) -> (u16, String, String) {
        let headers = self
            .retry_after
            .as_ref()
            .map_or_else(String::new, |v| format!("retry-after: {v}\r\n"));
        (self.status, headers, self.body.clone())
    }
}

// Connection scenarios get the same stack allowance as other repository scenarios.
fn on_fixture_thread(test: fn() -> TestResult) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || test().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "fixture thread panicked")?
        .map_err(Into::into)
}

fn fixture_runtime() -> Result<tokio::runtime::Runtime, std::io::Error> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}

fn fixture_login(attempt_ref: &AttemptRef) -> GatewayLogin<'_> {
    GatewayLogin {
        attempt_ref,
        character_id: CHARACTER,
        channel_id: None,
        client_build: "dev-1",
    }
}

fn assert_requests(server: &MockServer, count: usize) -> TestResult {
    let requests = server.requests.lock().map_err(|_| "request lock")?;
    assert_eq!(requests.len(), count);
    for request in requests.iter() {
        assert!(request.starts_with("POST /v1/login HTTP/1.1\r\n"));
        assert_eq!(request_body(request).as_bytes(), REQUEST.as_bytes());
    }
    Ok(())
}

fn assert_grant(grant: &NativeLoginGrant) -> TestResult {
    let expected: serde_json::Value = serde_json::from_str(SUCCESS)?;
    assert_eq!(grant.world_id, expected["world_id"]);
    assert_eq!(grant.channel_id, expected["channel_id"]);
    assert_eq!(grant.endpoint.host, expected["endpoint"]["host"]);
    assert_eq!(grant.endpoint.port, expected["endpoint"]["port"]);
    assert_eq!(
        grant.endpoint.tls_server_name,
        expected["endpoint"]["tls_server_name"]
    );
    assert_eq!(
        grant.grant_bytes(),
        expected["grant"]["token"]
            .as_str()
            .ok_or("token")?
            .as_bytes()
    );
    assert!(grant.usable_until > Instant::now());
    assert!(grant.usable_until <= Instant::now() + Duration::from_secs(30));
    Ok(())
}

#[test]
fn gateway_login_request_matches_platform_golden_bytes() -> TestResult {
    on_fixture_thread(|| {
        let runtime = fixture_runtime()?;
        let sequences: Vec<Vec<Vector>> = serde_json::from_str(SEQUENCES)?;
        let server = spawn_server_with_headers(
            &runtime,
            sequences[0].iter().map(Vector::response).collect(),
        )?;
        let client = client_for(&server)?;
        runtime.block_on(client.gateway_login(
            &ticket(),
            &fixture_login(&attempt()),
            CancellationToken::new(),
        ))?;
        assert_requests(&server, 3)?;
        let sorted: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/contracts/fixtures/platform-native-login-v1/request_sorted.json"
        ))?;
        assert_eq!(serde_json::from_str::<serde_json::Value>(REQUEST)?, sorted);
        Ok(())
    })
}

#[test]
fn gateway_login_parses_every_platform_success_vector() -> TestResult {
    on_fixture_thread(|| {
        let runtime = fixture_runtime()?;
        // success.json is one golden response; sequences contain its repeated use.
        let server =
            spawn_server_with_headers(&runtime, vec![(200, String::new(), SUCCESS.to_owned())])?;
        let grant = runtime.block_on(client_for(&server)?.gateway_login(
            &ticket(),
            &fixture_login(&attempt()),
            CancellationToken::new(),
        ))?;
        assert_grant(&grant)?;
        assert_eq!(server.requests.lock().map_err(|_| "request lock")?.len(), 1);
        Ok(())
    })
}

#[test]
fn gateway_login_maps_every_platform_error_vector() -> TestResult {
    on_fixture_thread(|| {
        let runtime = fixture_runtime()?;
        let vectors: Vec<Vector> = serde_json::from_str(ERRORS)?;
        for vector in &vectors {
            let expected: serde_json::Value = serde_json::from_str(&vector.body)?;
            let server = spawn_server_with_headers(&runtime, vec![vector.response()])?;
            let client = client_for(&server)?;
            // One HTTP exchange exposes retryable rows without consuming retries.
            let endpoint = client.gateway.join("v1/login")?;
            let (error, retry_after) = runtime
                .block_on(client.login_once(&endpoint, REQUEST.as_bytes(), &attempt()))
                .err()
                .ok_or("expected gateway error")?;
            let NativeLoginError::Gateway(code) = error else {
                return Err(format!("unexpected error for {}: {error:?}", vector.body).into());
            };
            assert_eq!(code.wire_code(), expected["error"]["code"]);
            assert_eq!(
                error.public_class(),
                PublicClass::parse(expected["error"]["public_class"].as_str().ok_or("class")?)
                    .ok_or("known class")?
            );
            assert_eq!(code.http_status(), vector.status);
            assert_eq!(
                format!("{:?}", code.progression()),
                vector.progression.as_deref().ok_or("progression")?
            );
            assert_eq!(retry_after, None);
        }
        // Boundary checks use the Platform rate-limited response with HTTP headers.
        let limited = vectors
            .iter()
            .find(|v| v.status == 429)
            .ok_or("rate limit vector")?;
        for (seconds, expected_requests) in [(30, 2), (31, 1)] {
            let server = spawn_server_with_headers(
                &runtime,
                vec![
                    (
                        limited.status,
                        format!("retry-after: {seconds}\r\n"),
                        limited.body.clone(),
                    ),
                    (200, String::new(), SUCCESS.to_owned()),
                ],
            )?;
            let client = client_for(&server)?.with_pacing(RetryPacing::CONTRACT);
            let started = Instant::now();
            let result = runtime.block_on(client.gateway_login(
                &ticket(),
                &fixture_login(&attempt()),
                CancellationToken::new(),
            ));
            if seconds == 30 {
                assert_grant(&result?)?;
                assert!(started.elapsed() >= Duration::from_secs(30));
            } else {
                assert_eq!(
                    result.map(|_| ()),
                    Err(NativeLoginError::Gateway(GatewayErrorCode::RateLimited))
                );
            }
            assert_eq!(
                server.requests.lock().map_err(|_| "request lock")?.len(),
                expected_requests
            );
        }
        Ok(())
    })
}

#[test]
fn gateway_login_rejects_every_platform_rejected_vector() -> TestResult {
    on_fixture_thread(|| {
        let runtime = fixture_runtime()?;
        let successes: Vec<String> = serde_json::from_str(include_str!(
            "../../../../docs/contracts/fixtures/platform-native-login-v1/rejected_successes.json"
        ))?;
        let errors: Vec<Vector> = serde_json::from_str(include_str!(
            "../../../../docs/contracts/fixtures/platform-native-login-v1/rejected_errors.json"
        ))?;
        for (status, headers, body) in successes
            .into_iter()
            .map(|body| (200, String::new(), body))
            .chain(errors.iter().map(Vector::response))
        {
            let server =
                spawn_server_with_headers(&runtime, vec![(status, headers, body.clone())])?;
            let result = runtime.block_on(client_for(&server)?.gateway_login(
                &ticket(),
                &fixture_login(&attempt()),
                CancellationToken::new(),
            ));
            assert_eq!(
                result.map(|_| ()),
                Err(NativeLoginError::Platform(
                    PlatformClientError::InvalidPayload
                )),
                "{body}"
            );
            assert_eq!(server.requests.lock().map_err(|_| "request lock")?.len(), 1);
        }
        Ok(())
    })
}

#[test]
fn gateway_login_follows_platform_sequences() -> TestResult {
    on_fixture_thread(|| {
        let runtime = fixture_runtime()?;
        let sequences: Vec<Vec<Vector>> = serde_json::from_str(SEQUENCES)?;
        assert_eq!(sequences.len(), 3);
        for (index, sequence) in sequences.iter().enumerate() {
            let server = spawn_server_with_headers(
                &runtime,
                sequence.iter().map(Vector::response).collect(),
            )?;
            let client = client_for(&server)?;
            let ticket = ticket();
            let attempt = attempt();
            let login = fixture_login(&attempt);
            let result =
                runtime.block_on(client.gateway_login(&ticket, &login, CancellationToken::new()));
            match index {
                0 => {
                    assert_grant(&result?)?;
                    assert_requests(&server, 3)?;
                }
                1 => {
                    assert_eq!(
                        result.map(|_| ()),
                        Err(NativeLoginError::Gateway(GatewayErrorCode::Unavailable))
                    );
                    assert_requests(&server, 6)?;
                    let result = runtime.block_on(client.gateway_login(
                        &ticket,
                        &login,
                        CancellationToken::new(),
                    ));
                    assert_eq!(result.map(|_| ()), Err(NativeLoginError::TicketSpent));
                    assert_requests(&server, 6)?;
                }
                2 => {
                    assert_eq!(
                        result.map(|_| ()),
                        Err(NativeLoginError::Gateway(
                            GatewayErrorCode::AuthenticationRequired
                        ))
                    );
                    assert_requests(&server, 1)?;
                }
                _ => return Err("unexpected sequence".into()),
            }
        }
        // Reconciliation after a lost admission result reuses the original ticket,
        // attempt, and budget, even across separate gateway_login invocations.
        let sequence = &sequences[0];
        let server = spawn_server_with_headers(
            &runtime,
            sequence
                .iter()
                .map(Vector::response)
                .chain(std::iter::once((200, String::new(), SUCCESS.to_owned())))
                .collect(),
        )?;
        let client = client_for(&server)?;
        let ticket = ticket();
        let attempt = attempt();
        for _ in 0..2 {
            assert_grant(&runtime.block_on(client.gateway_login(
                &ticket,
                &fixture_login(&attempt),
                CancellationToken::new(),
            ))?)?;
        }
        assert_eq!(ticket.requests_sent.load(Ordering::Relaxed), 4);
        assert_requests(&server, 4)?;
        Ok(())
    })
}
