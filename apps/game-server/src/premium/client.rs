//! PREM-1b: the Premium snapshot client (PREMIUM-DELIVERY-0 §3 and §3.1; §11 scope items 1-2).
//!
//! One `POST /v1/premium/snapshot` over mutual TLS to Platform's private endpoint: the client
//! identity and the Platform CA come from environment configuration and nothing else is trusted;
//! HTTPS only, no proxy, redirects never followed, a 5-second total timeout (`PREMDEL0-RL-03`)
//! and the body read capped at 1,024 bytes (`PREMDEL0-RL-01`) before parsing. Only a 200 with
//! `application/json` returns a body; anything else is a [`PullFailure`] (§3.1 "anything else is
//! unavailable"). Without configuration there is no client and Premium reads Free; login is
//! never affected.

use super::snapshot::{MAX_SNAPSHOT_BYTES, canonical_uuid, rfc3339_utc_micros};
use std::ffi::OsString;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const SNAPSHOT_PATH: &str = "/v1/premium/snapshot";
pub const REQUEST_SCHEMA: &str = "oteryn.premium_snapshot_request.v1";
/// `PREMDEL0-RL-02`.
pub const MAX_REQUEST_BYTES: usize = 256;
/// `PREMDEL0-RL-03`.
pub const PULL_TIMEOUT: Duration = Duration::from_secs(5);

/// The environment variables of the client configuration. The URL names the Platform endpoint's
/// origin (`https://host:port`); the two files are PEM: the Game server's client certificate
/// chain with its private key, and the Platform CA bundle. The credentials live only in the
/// environment (§3); none is committed.
pub const URL_VAR: &str = "OTERYN_PREMIUM_SNAPSHOT_URL";
pub const IDENTITY_VAR: &str = "OTERYN_PREMIUM_CLIENT_IDENTITY_PEM";
pub const PLATFORM_CA_VAR: &str = "OTERYN_PREMIUM_PLATFORM_CA_PEM";

#[derive(Clone)]
pub struct PremiumClientConfig {
    pub origin: String,
    pub identity_pem: Vec<u8>,
    pub platform_ca_pem: Vec<u8>,
}

impl std::fmt::Debug for PremiumClientConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PremiumClientConfig")
            .field("origin", &self.origin)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientConfigError {
    /// A variable is set but another is missing, a file is unreadable, or a value is invalid.
    Invalid(&'static str),
}

impl std::fmt::Display for ClientConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self::Invalid(what) = self;
        write!(f, "invalid Premium snapshot client configuration: {what}")
    }
}

impl std::error::Error for ClientConfigError {}

impl PremiumClientConfig {
    /// `None` when none of the three variables is set: no client, Premium reads Free. A partial
    /// configuration is an error, never silently unconfigured.
    pub fn from_env() -> Result<Option<Self>, ClientConfigError> {
        Self::from_vars(
            std::env::var_os(URL_VAR),
            std::env::var_os(IDENTITY_VAR),
            std::env::var_os(PLATFORM_CA_VAR),
        )
    }

    /// [`Self::from_env`] over given values: the origin and the two PEM file paths.
    pub fn from_vars(
        origin: Option<OsString>,
        identity_path: Option<OsString>,
        platform_ca_path: Option<OsString>,
    ) -> Result<Option<Self>, ClientConfigError> {
        let (origin, identity_path, platform_ca_path) =
            match (origin, identity_path, platform_ca_path) {
                (None, None, None) => return Ok(None),
                (Some(origin), Some(identity), Some(ca)) => (origin, identity, ca),
                (None, ..) => return Err(ClientConfigError::Invalid(URL_VAR)),
                (_, None, _) => return Err(ClientConfigError::Invalid(IDENTITY_VAR)),
                (.., None) => return Err(ClientConfigError::Invalid(PLATFORM_CA_VAR)),
            };
        let origin = origin
            .into_string()
            .map_err(|_| ClientConfigError::Invalid(URL_VAR))?;
        let file = |path: OsString, var: &'static str| {
            std::fs::read(path).map_err(|_| ClientConfigError::Invalid(var))
        };
        Ok(Some(Self {
            origin,
            identity_pem: file(identity_path, IDENTITY_VAR)?,
            platform_ca_pem: file(platform_ca_path, PLATFORM_CA_VAR)?,
        }))
    }
}

/// Why a pull yielded no body to ingest. Each is a failed pull (§3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullFailure {
    /// TLS, connection or I/O failure, or no fresh nonce.
    Transport,
    Timeout,
    /// A status other than 200, a redirect included. `retry_after` is a 429 or 503
    /// `Retry-After` in seconds.
    Status {
        status: u16,
        retry_after: Option<Duration>,
    },
    ContentType,
    Oversize,
}

impl PullFailure {
    pub fn retry_after(self) -> Option<Duration> {
        match self {
            Self::Status { retry_after, .. } => retry_after,
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PremiumSnapshotClient {
    http: reqwest::Client,
    endpoint: reqwest::Url,
}

impl PremiumSnapshotClient {
    pub fn new(config: &PremiumClientConfig) -> Result<Self, ClientConfigError> {
        let endpoint = reqwest::Url::parse(&config.origin)
            .ok()
            .filter(|url| {
                url.scheme() == "https"
                    && url.host().is_some()
                    && url.path() == "/"
                    && url.query().is_none()
                    && url.username().is_empty()
                    && url.password().is_none()
            })
            .and_then(|url| url.join(SNAPSHOT_PATH).ok())
            .ok_or(ClientConfigError::Invalid(URL_VAR))?;
        let identity = reqwest::Identity::from_pem(&config.identity_pem)
            .map_err(|_| ClientConfigError::Invalid(IDENTITY_VAR))?;
        let roots = reqwest::Certificate::from_pem_bundle(&config.platform_ca_pem)
            .ok()
            .filter(|roots| !roots.is_empty())
            .ok_or(ClientConfigError::Invalid(PLATFORM_CA_VAR))?;
        let http = reqwest::Client::builder()
            .https_only(true)
            .tls_certs_only(roots)
            .identity(identity)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(PULL_TIMEOUT)
            .build()
            .map_err(|_| ClientConfigError::Invalid(IDENTITY_VAR))?;
        Ok(Self { http, endpoint })
    }

    /// Pull the snapshot of `account_id` with `nonce`; the body is not yet validated.
    pub async fn pull(&self, account_id: [u8; 16], nonce: &str) -> Result<Vec<u8>, PullFailure> {
        tokio::time::timeout(PULL_TIMEOUT, self.exchange(account_id, nonce))
            .await
            .unwrap_or(Err(PullFailure::Timeout))
    }

    async fn exchange(&self, account_id: [u8; 16], nonce: &str) -> Result<Vec<u8>, PullFailure> {
        let body = request_body(account_id, nonce).ok_or(PullFailure::Transport)?;
        let mut response = self
            .http
            .post(self.endpoint.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await
            .map_err(transport)?;
        let status = response.status().as_u16();
        if status != 200 {
            let retry_after = matches!(status, 429 | 503)
                .then(|| retry_after(response.headers()))
                .flatten();
            return Err(PullFailure::Status {
                status,
                retry_after,
            });
        }
        if !is_json(response.headers()) {
            return Err(PullFailure::ContentType);
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_SNAPSHOT_BYTES as u64)
        {
            return Err(PullFailure::Oversize);
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport)? {
            if body.len() + chunk.len() > MAX_SNAPSHOT_BYTES {
                return Err(PullFailure::Oversize);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }
}

fn transport(error: reqwest::Error) -> PullFailure {
    if error.is_timeout() {
        PullFailure::Timeout
    } else {
        PullFailure::Transport
    }
}

/// The §3.1 request body, at most [`MAX_REQUEST_BYTES`].
pub fn request_body(account_id: [u8; 16], nonce: &str) -> Option<Vec<u8>> {
    let body = serde_json::to_vec(&serde_json::json!({
        "schema": REQUEST_SCHEMA,
        "account_id": canonical_uuid(account_id),
        "nonce": nonce,
    }))
    .ok()?;
    (body.len() <= MAX_REQUEST_BYTES).then_some(body)
}

/// A fresh 128-bit nonce as 32 lowercase hexadecimal characters, from the operating-system
/// seeded CSPRNG of the TLS provider (AWS-LC), as the gameplay seam's session identifiers. A new
/// one for every request, retries included.
pub fn fresh_nonce() -> Option<String> {
    let mut bytes = [0u8; 16];
    rustls::crypto::aws_lc_rs::default_provider()
        .secure_random
        .fill(&mut bytes)
        .ok()?;
    Some(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn is_json(headers: &reqwest::header::HeaderMap) -> bool {
    headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|essence| essence.trim().eq_ignore_ascii_case("application/json"))
}

/// The `Retry-After` of a response, either form; the caller caps it.
fn retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let now_us = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_micros()).ok())?;
    parse_retry_after(
        headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?,
        now_us,
    )
}

/// A `Retry-After` value (RFC 9110 §10.2.3) at `now_us`: delay seconds, or an IMF-fixdate
/// HTTP date (`Sun, 06 Nov 1994 08:49:37 GMT`) as the time until it, zero once it passed.
/// Any other form is `None` (the backoff alone applies).
pub fn parse_retry_after(value: &str, now_us: i64) -> Option<Duration> {
    let value = value.trim();
    if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
        return value.parse().ok().map(Duration::from_secs);
    }
    let parts: [&str; 6] = value.split(' ').collect::<Vec<_>>().try_into().ok()?;
    let [weekday, day, month, year, time, "GMT"] = parts else {
        return None;
    };
    const WEEKDAYS: [&str; 7] = ["Mon,", "Tue,", "Wed,", "Thu,", "Fri,", "Sat,", "Sun,"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month = MONTHS.iter().position(|m| *m == month)? + 1;
    if !WEEKDAYS.contains(&weekday) || day.len() != 2 || year.len() != 4 || time.len() != 8 {
        return None;
    }
    let at_us = rfc3339_utc_micros(&format!("{year}-{month:02}-{day}T{time}Z"))?;
    let wait = u64::try_from(at_us.saturating_sub(now_us)).unwrap_or(0);
    Some(Duration::from_micros(wait))
}
