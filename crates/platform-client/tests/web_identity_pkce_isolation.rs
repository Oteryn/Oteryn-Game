//! Real portal-rs journey, invoked only by tools/qualification/game_platform/run.sh.
//! No HTTP server, identity writer, or token producer is substituted here.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use oteryn_foundation::CancellationToken;
use oteryn_platform_client::{
    PlatformClientConfig,
    native_login::{NativeLoginClient, NativeLoginError},
};
use reqwest::{Client, Response, Url, header, redirect::Policy};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fs, process::Command, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

fn check(value: bool, reason: &'static str) -> TestResult {
    if value { Ok(()) } else { Err(reason.into()) }
}
fn text<'a>(value: &'a Value, key: &str) -> TestResult<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| "configuration_field_missing".into())
}
fn read_json(path: &str) -> TestResult<Value> {
    let bytes = fs::read(path).map_err(|_| "configuration_read_failed")?;
    serde_json::from_slice(&bytes).map_err(|_| "configuration_json_invalid".into())
}
fn random() -> TestResult<String> {
    let output = Command::new("openssl")
        .args(["rand", "-hex", "32"])
        .output()
        .map_err(|_| "random_source_unavailable")?;
    check(output.status.success(), "random_source_failed")?;
    let value = String::from_utf8(output.stdout).map_err(|_| "random_source_invalid")?;
    Ok(value.trim().to_owned())
}
fn digest(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
// psql is an independent read-only observer. SQL and database URLs never appear
// in argv, diagnostics, assertions, or returned errors.
fn sql(database: &str, query: &str) -> TestResult<String> {
    use std::{io::Write, process::Stdio};
    let mut child = Command::new("psql")
        .args(["-X", "-qAt", "-v", "ON_ERROR_STOP=1"])
        .env("PGDATABASE", database)
        .env("PGCONNECT_TIMEOUT", "5")
        .env(
            "PGOPTIONS",
            "-c default_transaction_read_only=on -c statement_timeout=3000",
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "postgres_observer_unavailable")?;
    child
        .stdin
        .take()
        .ok_or("postgres_stdin_missing")?
        .write_all(query.as_bytes())
        .map_err(|_| "postgres_query_write_failed")?;
    let output = child
        .wait_with_output()
        .map_err(|_| "postgres_observer_failed")?;
    check(output.status.success(), "postgres_assertion_query_failed")?;
    Ok(String::from_utf8(output.stdout)
        .map_err(|_| "postgres_output_invalid")?
        .trim()
        .to_owned())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Account {
    account_id: String,
    email: String,
    password: String,
}
#[derive(Deserialize)]
struct Input {
    accounts: [Account; 2],
    callback: String,
    client: String,
}

// Explicit cookie jar: only the session cookie is retained, always scoped to the
// exact configured public origin. reqwest's optional cookie feature is not needed.
struct Browser {
    client: Client,
    origin: Url,
    cookie: Option<String>,
}
impl Browser {
    fn new(origin: &str) -> TestResult<Self> {
        Ok(Self {
            client: Client::builder()
                .redirect(Policy::none())
                .no_proxy()
                .timeout(Duration::from_secs(8))
                .build()
                .map_err(|_| "browser_creation_failed")?,
            origin: Url::parse(origin).map_err(|_| "origin_invalid")?,
            cookie: None,
        })
    }
    async fn request(
        &mut self,
        path: &str,
        form: Option<&[(String, String)]>,
    ) -> TestResult<Response> {
        let url = self.origin.join(path).map_err(|_| "browser_url_invalid")?;
        check(url.origin() == self.origin.origin(), "cookie_origin_escape")?;
        let mut request = match form {
            Some(fields) => self
                .client
                .post(url)
                .header(header::ORIGIN, self.origin.origin().ascii_serialization())
                .form(fields),
            None => self.client.get(url),
        };
        if let Some(cookie) = &self.cookie {
            request = request.header(header::COOKIE, cookie);
        }
        let response = request.send().await.map_err(|_| "browser_request_failed")?;
        for cookie in response.headers().get_all(header::SET_COOKIE) {
            let pair = cookie
                .to_str()
                .map_err(|_| "cookie_invalid")?
                .split(';')
                .next()
                .ok_or("cookie_missing")?;
            if pair.starts_with("__Host-oteryn_session=") {
                self.cookie = Some(pair.to_owned());
            }
        }
        Ok(response)
    }
    async fn csrf(&mut self, path: &str) -> TestResult<String> {
        let response = self.request(path, None).await?;
        if response.status() == 404 {
            let source = if path.starts_with("/oauth/") {
                "portal-rs/crates/gameauth/src/oauth/oauth_http.rs:27"
            } else {
                "portal-rs/crates/identity/src/login_http.rs:35"
            };
            if let Ok(file) = std::env::var("GAME_PLATFORM_RESULT_FILE") {
                fs::write(
                    file,
                    format!("GAME_PLATFORM_RESULT=BLOCKED owner=Platform what={source}\n"),
                )
                .map_err(|_| "result_write_failed")?;
            }
        }
        check(response.status() == 200, "form_handler_missing_or_refused")?;
        let body = response.text().await.map_err(|_| "form_read_failed")?;
        let token = body
            .split("name=\"_csrf\" value=\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .ok_or("csrf_form_missing")?;
        check(
            token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit()),
            "csrf_form_invalid",
        )?;
        Ok(token.to_owned())
    }
    async fn login(
        &mut self,
        account: &Account,
        password: &str,
        csrf: Option<&str>,
    ) -> TestResult<Response> {
        let mut fields = vec![
            ("email".into(), account.email.clone()),
            ("password".into(), password.into()),
        ];
        if let Some(csrf) = csrf {
            fields.push(("_csrf".into(), csrf.into()));
        }
        self.request("/en/login", Some(&fields)).await
    }
}
fn location(response: &Response) -> TestResult<&str> {
    response
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| "redirect_missing".into())
}
fn cookie_hash(browser: &Browser) -> TestResult<String> {
    let cookie = browser.cookie.as_deref().ok_or("session_cookie_missing")?;
    Ok(digest(
        cookie.split_once('=').ok_or("session_cookie_invalid")?.1,
    ))
}
fn session_owner(database: &str, browser: &Browser) -> TestResult<String> {
    sql(
        database,
        &format!(
            "SELECT COALESCE(account_id::text,'anonymous') FROM web_sessions WHERE id_hash=decode('{}','hex') AND idle_expires_at>now() AND absolute_expires_at>now();",
            cookie_hash(browser)?
        ),
    )
}
fn callback_code(url: &Url, expected_state: &str, callback: &str) -> TestResult<String> {
    let expected = Url::parse(callback).map_err(|_| "callback_configuration_invalid")?;
    check(
        url.origin() == expected.origin() && url.path() == expected.path(),
        "callback_destination_mismatch",
    )?;
    let pairs: Vec<_> = url.query_pairs().collect();
    check(
        pairs.len() == 2
            && pairs
                .iter()
                .filter(|(k, v)| k == "state" && v == expected_state)
                .count()
                == 1,
        "callback_state_mismatch",
    )?;
    pairs
        .iter()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| "callback_code_missing".into())
}
async fn authorize(
    browser: &mut Browser,
    native: &NativeLoginClient,
    input: &Input,
    listener: &TcpListener,
    state: &str,
    verifier: &str,
) -> TestResult<String> {
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let url = native
        .authorization_url(&input.callback, state, &challenge)
        .map_err(|_| "authorization_url_failed")?;
    let path = format!(
        "{}?{}",
        url.path(),
        url.query().ok_or("authorization_query_missing")?
    );
    let csrf = browser.csrf(&path).await?;
    let mut fields: BTreeMap<String, String> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    fields.insert("_csrf".into(), csrf);
    fields.insert("decision".into(), "approve".into());
    let response = browser
        .request(
            "/oauth/authorize",
            Some(&fields.into_iter().collect::<Vec<_>>()),
        )
        .await?;
    check(response.status().is_redirection(), "consent_refused")?;
    let redirect = Url::parse(location(&response)?).map_err(|_| "callback_redirect_invalid")?;
    let mut bad = redirect.clone();
    let actual_code = callback_code(&redirect, state, &input.callback)?;
    bad.query_pairs_mut()
        .clear()
        .append_pair("code", &actual_code)
        .append_pair("state", &random()?);
    // Both state refusal and success cross the actual loopback HTTP boundary.
    // Only the state changes; neither callback can exchange a token itself.
    let send = async {
        let refused = browser
            .client
            .get(bad)
            .send()
            .await
            .map_err(|_| "loopback_request_failed")?;
        check(refused.status() == 400, "state_mismatch_accepted")?;
        let accepted = browser
            .client
            .get(redirect)
            .send()
            .await
            .map_err(|_| "loopback_request_failed")?;
        check(accepted.status() == 200, "loopback_refused")
    };
    let receive = async {
        let mut accepted_code = None;
        for expect_valid in [false, true] {
            let (mut socket, peer) = listener
                .accept()
                .await
                .map_err(|_| "loopback_accept_failed")?;
            check(peer.ip().is_loopback(), "non_loopback_callback")?;
            let mut bytes = vec![0; 8192];
            let mut used = 0;
            while !bytes[..used].windows(4).any(|w| w == b"\r\n\r\n") {
                check(used < bytes.len(), "loopback_request_too_large")?;
                let count = socket
                    .read(&mut bytes[used..])
                    .await
                    .map_err(|_| "loopback_read_failed")?;
                check(count > 0, "loopback_request_incomplete")?;
                used += count;
            }
            let request =
                std::str::from_utf8(&bytes[..used]).map_err(|_| "loopback_http_invalid")?;
            let target = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .ok_or("loopback_target_missing")?;
            let url = Url::parse(&input.callback)
                .map_err(|_| "callback_invalid")?
                .join(target)
                .map_err(|_| "callback_target_invalid")?;
            let code = callback_code(&url, state, &input.callback);
            check(
                code.is_ok() == expect_valid,
                "callback_state_validation_failed",
            )?;
            let response = if expect_valid {
                accepted_code = Some(code?);
                b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".as_slice()
            } else {
                b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .as_slice()
            };
            socket
                .write_all(response)
                .await
                .map_err(|_| "loopback_write_failed")?;
        }
        accepted_code.ok_or_else(|| "loopback_code_missing".into())
    };
    let (_, code) = futures::future::try_join(send, receive).await?;
    Ok(code)
}
async fn journey(input: Input, report: Value, game_database: String) -> TestResult {
    for account in &input.accounts {
        check(
            account.account_id.len() == 36
                && account
                    .account_id
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() || b == b'-'),
            "account_id_invalid",
        )?;
    }
    let portal_env = read_json(&format!(
        "{}/secrets/env.json",
        text(&report, "outputDirectory")?
    ))?;
    let database = text(&portal_env, "OTERYN_DATABASE_URL")?;
    check(database != game_database, "shared_database_forbidden")?;
    let public = text(&report["baseUrls"], "public")?;
    let config = PlatformClientConfig::new(public).map_err(|_| "native_config_invalid")?;
    let gateway = PlatformClientConfig::new(text(&report["baseUrls"], "gateway")?)
        .map_err(|_| "gateway_config_invalid")?;
    let native = NativeLoginClient::new(&config, &gateway, &input.client)
        .map_err(|_| "native_client_creation_failed")?;
    let callback = Url::parse(&input.callback).map_err(|_| "callback_invalid")?;
    let listener =
        TcpListener::bind(("127.0.0.1", callback.port().ok_or("callback_port_missing")?))
            .await
            .map_err(|_| "loopback_bind_failed")?;
    let mut a = Browser::new(public)?;
    let csrf = a.csrf("/en/login").await?;
    let anonymous = cookie_hash(&a)?;
    let response = a.login(&input.accounts[0], &random()?, Some(&csrf)).await?;
    check(
        response.status().is_redirection() && location(&response)? == "/en/login",
        "wrong_password_not_generic",
    )?;
    check(
        session_owner(database, &a)? == "anonymous",
        "wrong_password_created_authenticated_session",
    )?;
    let failed = a
        .request("/en/login", None)
        .await?
        .text()
        .await
        .map_err(|_| "failure_page_read_failed")?;
    check(
        failed.contains("role=\"alert\"") && !failed.contains(&input.accounts[0].email),
        "failure_page_not_generic",
    )?;
    for token in [None, Some("invalid")] {
        check(
            a.login(&input.accounts[0], &input.accounts[0].password, token)
                .await?
                .status()
                == 403,
            "csrf_not_rejected",
        )?;
        check(
            session_owner(database, &a)? == "anonymous",
            "csrf_created_session",
        )?;
    }
    let csrf = a.csrf("/en/login").await?;
    check(
        a.login(&input.accounts[0], &input.accounts[0].password, Some(&csrf))
            .await?
            .status()
            .is_redirection(),
        "real_login_refused",
    )?;
    check(
        cookie_hash(&a)? != anonymous,
        "login_did_not_rotate_session",
    )?;
    check(
        session_owner(database, &a)? == input.accounts[0].account_id,
        "wrong_account_session",
    )?;
    check(
        sql(
            database,
            &format!(
                "SELECT count(*) FROM web_sessions WHERE id_hash=decode('{anonymous}','hex');"
            ),
        )? == "0",
        "old_session_survived_login",
    )?;
    let verifier = random()?;
    let code = authorize(&mut a, &native, &input, &listener, &random()?, &verifier).await?;
    check(
        matches!(
            native
                .exchange_code(&code, &random()?, &input.callback, CancellationToken::new())
                .await,
            Err(NativeLoginError::TokenRefused)
        ),
        "pkce_mismatch_accepted",
    )?;
    check(
        sql(
            database,
            &format!(
                "SELECT count(*) FROM oauth_authorization_codes WHERE code_hash=decode('{}','hex') AND used_at IS NULL;",
                digest(&code)
            ),
        )? == "1",
        "negative_case_consumed_code",
    )?;
    check(
        sql(database, "SELECT count(*) FROM oauth_tokens;")? == "0",
        "negative_case_issued_token",
    )?;
    let _token = native
        .exchange_code(&code, &verifier, &input.callback, CancellationToken::new())
        .await
        .map_err(|_| "native_code_exchange_refused")?;
    check(
        matches!(
            native
                .exchange_code(&code, &verifier, &input.callback, CancellationToken::new())
                .await,
            Err(NativeLoginError::TokenRefused)
        ),
        "authorization_code_reused",
    )?;
    check(
        sql(
            database,
            &format!(
                "SELECT count(*) FROM oauth_tokens t JOIN oauth_token_families f ON f.id=t.family_id WHERE f.account_id='{}';",
                input.accounts[0].account_id
            ),
        )? == "1",
        "token_not_bound_to_account_a",
    )?;
    check(
        sql(
            database,
            &format!(
                "SELECT count(*) FROM oauth_token_families WHERE account_id='{}';",
                input.accounts[1].account_id
            ),
        )? == "0",
        "account_b_received_account_a_token",
    )?;
    // Another account's credentials cannot relabel an authenticated jar.
    let csrf = csrf_for_logout(&mut a, &native, &input).await?;
    a.login(&input.accounts[1], &input.accounts[1].password, Some(&csrf))
        .await?;
    check(
        session_owner(database, &a)? == input.accounts[0].account_id,
        "account_b_rebound_account_a_session",
    )?;
    let mut b = Browser::new(public)?;
    let csrf = b.csrf("/en/login").await?;
    b.login(&input.accounts[1], &input.accounts[1].password, Some(&csrf))
        .await?;
    check(
        session_owner(database, &b)? == input.accounts[1].account_id,
        "account_b_login_not_isolated",
    )?;
    let old_a_cookie = a.cookie.clone();
    let old_a_hash = cookie_hash(&a)?;
    let csrf = csrf_for_logout(&mut a, &native, &input).await?;
    check(
        a.request("/en/logout", Some(&[("_csrf".into(), csrf)]))
            .await?
            .status()
            .is_redirection(),
        "logout_refused",
    )?;
    check(
        cookie_hash(&a)? != old_a_hash,
        "logout_did_not_rotate_session",
    )?;
    check(
        session_owner(database, &a)? == "anonymous",
        "logout_kept_authenticated_session",
    )?;
    let mut replay = Browser::new(public)?;
    replay.cookie = old_a_cookie;
    check(
        session_owner(database, &replay)?.is_empty(),
        "logout_session_row_survived",
    )?;
    let response = replay.request("/en/login", None).await?;
    check(
        response.status() == 200 && session_owner(database, &replay)? == "anonymous",
        "logout_cookie_replay_authenticated",
    )?;
    check(
        session_owner(database, &b)? == input.accounts[1].account_id,
        "logout_crossed_accounts",
    )?;
    check(
        sql(
            &game_database,
            "SELECT count(*) FROM game_durability_fresh_admission_receipts;",
        )? == "0",
        "identity_admitted_to_game",
    )?;
    println!("NODE_BOOT_EVIDENCE flow=web_identity result=pass account_isolation=1");
    Ok(())
}
async fn csrf_for_logout(
    browser: &mut Browser,
    native: &NativeLoginClient,
    input: &Input,
) -> TestResult<String> {
    let url = native
        .authorization_url(
            &input.callback,
            &random()?,
            &URL_SAFE_NO_PAD.encode(Sha256::digest(random()?.as_bytes())),
        )
        .map_err(|_| "authorization_url_failed")?;
    browser
        .csrf(&format!(
            "/oauth/authorize?{}",
            url.query().ok_or("authorization_query_missing")?
        ))
        .await
}
#[test]
fn web_identity_pkce_isolation() -> TestResult {
    let Ok(input_file) = std::env::var("GAME_PLATFORM_INPUT") else {
        // Ordinary crate tests have no external topology and emit no E2E evidence.
        return Ok(());
    };
    let result = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> TestResult {
            let input: Input = serde_json::from_value(read_json(&input_file)?)
                .map_err(|_| "journey_input_invalid")?;
            let report = read_json(
                &std::env::var("GAME_PLATFORM_REPORT").map_err(|_| "report_not_configured")?,
            )?;
            let game = std::env::var("GAME_PLATFORM_GAME_DATABASE_URL")
                .map_err(|_| "game_database_not_configured")?;
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| "journey_runtime_failed")?
                .block_on(async {
                    tokio::time::timeout(Duration::from_secs(60), journey(input, report, game))
                        .await
                        .map_err(|_| "journey_budget_exceeded")?
                })
        })
        .map_err(|_| "journey_thread_failed")?
        .join()
        .map_err(|_| "journey_thread_panicked")?;
    if let Err(error) = &result
        && let Ok(file) = std::env::var("GAME_PLATFORM_RESULT_FILE")
        && !std::path::Path::new(&file).exists()
    {
        let reason = error.to_string();
        check(
            reason.len() <= 128 && reason.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
            "invalid_failure_reason",
        )?;
        fs::write(
            file,
            format!("GAME_PLATFORM_RESULT=FAIL flow=web_identity reason={reason}\n"),
        )
        .map_err(|_| "result_write_failed")?;
    }
    result
}
