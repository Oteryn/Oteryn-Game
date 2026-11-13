//! Real portal-rs journey, invoked only by tools/qualification/game_platform/run.sh.
//! No HTTP server, identity writer, or token producer is substituted here.
// Each integration target uses a different subset of these shared journey helpers.
#![allow(dead_code)]
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use oteryn_foundation::CancellationToken;
use oteryn_identity::{IdentityError, IdentityFlow, StateNonce};
use oteryn_platform_client::native_login::NativeLoginClient;
use reqwest::{Client, Response, Url, header, redirect::Policy};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fs, process::Command, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

pub type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

pub fn check(value: bool, reason: &'static str) -> TestResult {
    if value { Ok(()) } else { Err(reason.into()) }
}
pub fn text<'a>(value: &'a Value, key: &str) -> TestResult<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| "configuration_field_missing".into())
}
pub fn read_json(path: &str) -> TestResult<Value> {
    let bytes = fs::read(path).map_err(|_| "configuration_read_failed")?;
    serde_json::from_slice(&bytes).map_err(|_| "configuration_json_invalid".into())
}
pub fn random() -> TestResult<String> {
    let output = Command::new("openssl")
        .args(["rand", "-hex", "32"])
        .output()
        .map_err(|_| "random_source_unavailable")?;
    check(output.status.success(), "random_source_failed")?;
    let value = String::from_utf8(output.stdout).map_err(|_| "random_source_invalid")?;
    Ok(value.trim().to_owned())
}
pub fn digest(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
// URI components are not form data: literal '&' and '+' must survive.
pub fn pg_component(value: &str) -> TestResult<String> {
    let mut decoded = Vec::new();
    let mut bytes = value.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let high = bytes.next().and_then(|b| char::from(b).to_digit(16));
            let low = bytes.next().and_then(|b| char::from(b).to_digit(16));
            let (Some(high), Some(low)) = (high, low) else {
                return Err("postgres_component_invalid".into());
            };
            decoded.push((high * 16 + low) as u8);
        } else {
            decoded.push(byte);
        }
    }
    String::from_utf8(decoded).map_err(|_| "postgres_component_invalid".into())
}
pub fn postgres_env(database: &str) -> TestResult<Vec<(&'static str, String)>> {
    let url = Url::parse(database).map_err(|_| "postgres_url_invalid")?;
    check(
        matches!(url.scheme(), "postgres" | "postgresql"),
        "postgres_scheme_invalid",
    )?;
    Ok(vec![
        (
            "PGHOST",
            url.host_str()
                .ok_or("postgres_host_missing")?
                .trim_matches(['[', ']'])
                .to_owned(),
        ),
        ("PGPORT", url.port().unwrap_or(5432).to_string()),
        ("PGUSER", pg_component(url.username())?),
        (
            "PGPASSWORD",
            pg_component(url.password().unwrap_or_default())?,
        ),
        (
            "PGDATABASE",
            pg_component(
                url.path()
                    .strip_prefix('/')
                    .ok_or("postgres_database_missing")?,
            )?,
        ),
        (
            "PGSSLMODE",
            url.query_pairs()
                .find(|(key, _)| key == "sslmode")
                .map(|(_, value)| value.into_owned())
                .unwrap_or_else(|| "prefer".into()),
        ),
    ])
}
pub fn postgres_failure(label: &'static str, stderr: &[u8]) -> String {
    // LC_ALL=C and VERBOSITY=sqlstate produce ERROR/FATAL: <five-character code>.
    // Never forward message text, connection details, queries or arbitrary words.
    let state = std::str::from_utf8(stderr).ok().and_then(|text| {
        text.lines().find_map(|line| {
            let (_, rest) = line
                .split_once("ERROR:")
                .or_else(|| line.split_once("FATAL:"))?;
            let code = rest.trim();
            (code.len() == 5
                && code
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()))
            .then_some(code)
        })
    });
    let mut reason = format!("postgres_assertion_query_failed:{label}");
    if let Some(state) = state {
        reason.push(':');
        reason.push_str(state);
    }
    reason
}
#[test]
pub fn postgres_observer_connection_fields() -> TestResult {
    let fields = postgres_env(
        "postgresql://obs&erver:p%40ss%3A%2F%25+wo&rd@127.0.0.1:15435/game%20d&b?sslmode=require",
    )?;
    for (key, expected) in [
        ("PGHOST", "127.0.0.1"),
        ("PGPORT", "15435"),
        ("PGUSER", "obs&erver"),
        ("PGPASSWORD", "p@ss:/%+wo&rd"),
        ("PGDATABASE", "game d&b"),
        ("PGSSLMODE", "require"),
    ] {
        check(
            fields
                .iter()
                .any(|(name, value)| *name == key && value == expected),
            "postgres_field_mismatch",
        )?;
    }
    check(
        postgres_env("postgresql://observer@localhost/game")?
            .iter()
            .any(|(name, value)| *name == "PGSSLMODE" && value == "prefer"),
        "postgres_default_sslmode_invalid",
    )?;
    check(
        postgres_failure("probe", b"ERROR:  42P01\n")
            == "postgres_assertion_query_failed:probe:42P01",
        "postgres_sqlstate_missing",
    )?;
    check(
        postgres_failure("probe", b"connection failed: private connection details")
            == "postgres_assertion_query_failed:probe",
        "postgres_error_text_leaked",
    )
}
#[test]
pub fn postgres_observer_round_trip() -> TestResult {
    let Ok(database) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    check(
        sql(&database, "round_trip", "SELECT 1;")? == "1",
        "postgres_round_trip_failed",
    )?;
    let error = sql(
        &database,
        "invalid_query",
        "SELECT missing_observer_column;",
    )
    .err()
    .ok_or("postgres_invalid_query_accepted")?;
    check(
        error.to_string() == "postgres_assertion_query_failed:invalid_query:42703",
        "postgres_sqlstate_invalid",
    )
}
// psql is an independent read-only observer. SQL and database URLs never appear
// in argv, diagnostics, assertions, or returned errors.
pub fn sql(database: &str, label: &'static str, query: &str) -> TestResult<String> {
    use std::{io::Write, process::Stdio};
    let mut child = Command::new("psql")
        .args([
            "-X",
            "-qAt",
            "-v",
            "ON_ERROR_STOP=1",
            "-v",
            "VERBOSITY=sqlstate",
        ])
        .envs(postgres_env(database)?)
        .env("LC_ALL", "C")
        .env("PGCONNECT_TIMEOUT", "5")
        .env(
            "PGOPTIONS",
            "-c default_transaction_read_only=on -c statement_timeout=3000",
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| postgres_failure(label, &[]))?;
    child
        .stdin
        .take()
        .ok_or_else(|| postgres_failure(label, &[]))?
        .write_all(query.as_bytes())
        .map_err(|_| postgres_failure(label, &[]))?;
    let output = child
        .wait_with_output()
        .map_err(|_| postgres_failure(label, &[]))?;
    if !output.status.success() {
        return Err(postgres_failure(label, &output.stderr).into());
    }
    Ok(String::from_utf8(output.stdout)
        .map_err(|_| postgres_failure(label, &[]))?
        .trim()
        .to_owned())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub account_id: String,
    pub email: String,
    pub password: String,
}
#[derive(Deserialize)]
pub struct Input {
    #[serde(default)]
    pub native_origin: String,
    #[serde(default)]
    pub portal_binary: String,
    pub accounts: [Account; 2],
    pub callback: String,
    pub client: String,
}

// Explicit cookie jar: only the session cookie is retained, always scoped to the
// exact configured public origin. reqwest's optional cookie feature is not needed.
pub struct Browser {
    pub client: Client,
    pub origin: Url,
    pub cookie: Option<String>,
}
impl Browser {
    pub fn new(origin: &str) -> TestResult<Self> {
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
    pub async fn request(
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
    pub async fn csrf(&mut self, path: &str) -> TestResult<String> {
        let response = self.request(path, None).await?;
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
    pub async fn login(
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
pub fn location(response: &Response) -> TestResult<&str> {
    response
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| "redirect_missing".into())
}
pub fn cookie_hash(browser: &Browser) -> TestResult<String> {
    let cookie = browser.cookie.as_deref().ok_or("session_cookie_missing")?;
    Ok(digest(
        cookie.split_once('=').ok_or("session_cookie_invalid")?.1,
    ))
}
pub fn session_owner(database: &str, browser: &Browser) -> TestResult<String> {
    sql(
        database,
        "session_owner",
        &format!(
            "SELECT COALESCE((SELECT account_id::text FROM web_sessions WHERE id_hash=decode('{}','hex') AND idle_expires_at>now() AND absolute_expires_at>now()),'anonymous');",
            cookie_hash(browser)?
        ),
    )
}
pub async fn callback_code(url: &Url, expected_state: &str, callback: &str) -> TestResult<String> {
    let expected = Url::parse(callback).map_err(|_| "callback_configuration_invalid")?;
    check(
        url.origin() == expected.origin() && url.path() == expected.path(),
        "callback_destination_mismatch",
    )?;
    check(url.query_pairs().count() == 2, "callback_query_invalid")?;
    let flow = IdentityFlow::new(StateNonce::new(expected_state)?, Duration::from_secs(5));
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    sender
        .send(url.query().ok_or("callback_query_missing")?.to_owned())
        .await
        .map_err(|_| "callback_channel_closed")?;
    let code = flow
        .await_callback(&mut receiver, CancellationToken::new())
        .await?;
    Ok(code.expose().expose().to_owned())
}
#[test]
pub fn shipping_callback_rejects_mismatched_state() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let callback = "http://127.0.0.1:18099/callback";
            let bad = Url::parse(&format!("{callback}?code=qualification-code&state=wrong"))?;
            let error = callback_code(&bad, "expected", callback)
                .await
                .err()
                .ok_or("state_mismatch_accepted")?;
            check(
                error.downcast_ref::<IdentityError>() == Some(&IdentityError::StateMismatch),
                "wrong_state_refusal",
            )?;
            let good = Url::parse(&format!(
                "{callback}?code=qualification-code&state=expected"
            ))?;
            check(
                callback_code(&good, "expected", callback).await? == "qualification-code",
                "valid_callback_refused",
            )
        })
}
pub async fn authorize(
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
    let actual_code = callback_code(&redirect, state, &input.callback).await?;
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
            let code = callback_code(&url, state, &input.callback).await;
            if !expect_valid {
                check(
                    code.as_ref()
                        .err()
                        .and_then(|error| error.downcast_ref::<IdentityError>())
                        == Some(&IdentityError::StateMismatch),
                    "wrong_state_refusal",
                )?;
            }
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

pub struct Topology {
    pub input: Input,
    pub report: Value,
    pub database: String,
    pub game_database: String,
    browsers: tokio::sync::Mutex<BTreeMap<usize, Browser>>,
}
impl Topology {
    pub fn load() -> TestResult<Self> {
        let input: Input = serde_json::from_value(read_json(
            &std::env::var("GAME_PLATFORM_INPUT").map_err(|_| "input_missing")?,
        )?)
        .map_err(|_| "input_invalid")?;
        for account in &input.accounts {
            check(
                oteryn_platform_client::device_sessions::canonical_uuid(&account.account_id),
                "account_id_invalid",
            )?;
        }
        let report =
            read_json(&std::env::var("GAME_PLATFORM_REPORT").map_err(|_| "report_missing")?)?;
        let env = read_json(&format!(
            "{}/secrets/env.json",
            text(&report, "outputDirectory")?
        ))?;
        Ok(Self {
            database: text(&env, "OTERYN_DATABASE_URL")?.into(),
            game_database: std::env::var("GAME_PLATFORM_GAME_DATABASE_URL")
                .map_err(|_| "game_database_missing")?,
            input,
            report,
            browsers: Default::default(),
        })
    }
    pub fn native(&self, origin: &str) -> TestResult<NativeLoginClient> {
        let platform = oteryn_platform_client::PlatformClientConfig::new(origin)
            .map_err(|_| "platform_config_invalid")?;
        let gateway = oteryn_platform_client::PlatformClientConfig::new(text(
            &self.report["baseUrls"],
            "gateway",
        )?)
        .map_err(|_| "gateway_config_invalid")?;
        NativeLoginClient::new(&platform, &gateway, &self.input.client)
            .map_err(|_| "native_client_failed".into())
    }
    pub async fn code(&self, index: usize) -> TestResult<(String, String)> {
        let public = text(&self.report["baseUrls"], "public")?;
        let native = self.native(public)?;
        let account = &self.input.accounts[index];
        // One password login per account in this journey. Subsequent OAuth
        // codes use the authenticated session, not the five-per-minute limiter.
        let mut browsers = self.browsers.lock().await;
        if let std::collections::btree_map::Entry::Vacant(entry) = browsers.entry(index) {
            let mut browser = Browser::new(public)?;
            let csrf = browser.csrf("/en/login").await?;
            check(
                browser
                    .login(account, &account.password, Some(&csrf))
                    .await?
                    .status()
                    .is_redirection(),
                "oauth_login_refused",
            )?;
            entry.insert(browser);
        }
        let browser = browsers.get_mut(&index).ok_or("oauth_browser_missing")?;
        check(
            session_owner(&self.database, browser)? == account.account_id,
            "oauth_account_mismatch",
        )?;
        let url = Url::parse(&self.input.callback).map_err(|_| "callback_invalid")?;
        let listener = TcpListener::bind(("127.0.0.1", url.port().ok_or("callback_port_missing")?))
            .await
            .map_err(|_| "callback_bind_failed")?;
        let verifier = random()?;
        let code = authorize(
            browser,
            &native,
            &self.input,
            &listener,
            &random()?,
            &verifier,
        )
        .await?;
        Ok((code, verifier))
    }
    pub async fn oauth(
        &self,
        index: usize,
    ) -> TestResult<oteryn_platform_client::native_login::AccessToken> {
        let (code, verifier) = self.code(index).await?;
        self.native(text(&self.report["baseUrls"], "public")?)?
            .exchange_code(
                &code,
                &verifier,
                &self.input.callback,
                CancellationToken::new(),
            )
            .await
            .map_err(|_| "oauth_exchange_refused".into())
    }
    pub fn no_admissions(&self) -> TestResult {
        check(
            sql(
                &self.game_database,
                "game_admissions",
                "SELECT count(*) FROM game_durability_fresh_admission_receipts;",
            )? == "0",
            "unexpected_game_admission",
        )?;
        check(
            sql(
                &self.database,
                "platform_admissions",
                "SELECT count(*) FROM native_admissions;",
            )? == "0",
            "unexpected_platform_admission",
        )
    }
}

pub fn run<F, Fut>(flow: &'static str, seconds: u64, action: F) -> TestResult
where
    F: FnOnce(Topology) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = TestResult>,
{
    if std::env::var_os("GAME_PLATFORM_INPUT").is_none() {
        return Ok(());
    }
    let result = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> TestResult {
            let topology = Topology::load()?;
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| "runtime_failed")?
                .block_on(async {
                    tokio::time::timeout(Duration::from_secs(seconds), action(topology))
                        .await
                        .map_err(|_| "journey_budget_exceeded")?
                })
        })
        .map_err(|_| "thread_failed")?
        .join()
        .map_err(|_| "thread_panicked")?;
    if let Err(error) = &result {
        let reason = error.to_string();
        let reason = if reason.len() <= 128
            && reason
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
        {
            &reason
        } else {
            "journey_assertion_failed"
        };
        if let Ok(path) = std::env::var("GAME_PLATFORM_RESULT_FILE") {
            fs::write(
                path,
                format!("GAME_PLATFORM_RESULT=FAIL flow={flow} reason={reason}\n"),
            )
            .map_err(|_| "result_write_failed")?;
        }
        // Do not let Rust's test harness render arbitrary errors containing inputs.
        return Err(reason.to_owned().into());
    }
    result
}
