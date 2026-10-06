//! Native login against Platform and the Gateway (N4-P contract at Platform `f880cd74`,
//! ARCH-LOGIN-FIRST-0 §2.4): OAuth authorization code + PKCE S256 URL and token exchange, native
//! ticket issuance, and the Gateway `POST /v1/login` protocol-2 branch.
//!
//! Tokens, tickets and grants live only in [`SecretText`] values: in memory, redacted in `Debug`
//! and `Display`, never logged. This module decodes its own typed Gateway response, so it does not
//! pass through the directory's forbidden-field filter; nothing else may.

use crate::{PlatformClientConfig, PlatformClientError};
use oteryn_foundation::{Cancellable, CancellationToken, cancellable};
use reqwest::header::{CONTENT_TYPE, RETRY_AFTER};
use reqwest::{Client, Response, StatusCode, Url, redirect::Policy};
use serde::{Deserialize, Deserializer};
use std::fmt::{self, Debug, Display, Formatter};
use std::net::IpAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The only OAuth scope the native client requests (N4-P §2 step 1).
pub const NATIVE_LOGIN_SCOPE: &str = "game:ticket";
/// The Gateway native branch version (N4-P §3.1).
pub const GATEWAY_PROTOCOL_VERSION: u64 = 2;
/// The only transport offered and accepted in v1 (N4-P §3.1).
pub const GAME_ALPN: &str = "oteryn-game/1";
pub const GRANT_PROFILE: &str = "oteryn-pre-admission-v1";
/// Gateway request and response body bounds (N4-P §10).
pub const MAX_LOGIN_REQUEST_BYTES: usize = 2048;
pub const MAX_LOGIN_RESPONSE_BYTES: usize = 6144;
/// Total Gateway requests per (`attempt_ref`, ticket) (N4-P §10).
pub const MAX_LOGIN_REQUESTS: u32 = 6;
/// A native ticket lives at most this long; the client never extends it (N4-P §4).
pub const MAX_TICKET_LIFETIME: Duration = Duration::from_secs(60);
const MAX_OAUTH_RESPONSE_BYTES: usize = 16_384;
const MAX_TICKET_RESPONSE_BYTES: usize = 4096;
const MAX_ACCESS_TOKEN_BYTES: usize = 8192;
const MAX_TICKET_BYTES: usize = 256;
const MAX_GRANT_TOKEN_BYTES: usize = 4096;
const MAX_RETRY_AFTER: Duration = Duration::from_secs(30);

/// A credential held in memory only. `Debug` and `Display` never show it.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretText(String);

impl SecretText {
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl Debug for SecretText {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretText([REDACTED])")
    }
}

impl Display for SecretText {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

/// FND-04A public class: what the player is told, and the only thing the client acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicClass {
    RetryLogin,
    AuthenticationRequired,
    TemporarilyUnavailable,
    SessionUnavailable,
    ClientUpdateRequired,
    CharacterAlreadyActive,
}

impl PublicClass {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "RETRY_LOGIN" => Self::RetryLogin,
            "AUTHENTICATION_REQUIRED" => Self::AuthenticationRequired,
            "TEMPORARILY_UNAVAILABLE" => Self::TemporarilyUnavailable,
            "SESSION_UNAVAILABLE" => Self::SessionUnavailable,
            "CLIENT_UPDATE_REQUIRED" => Self::ClientUpdateRequired,
            "CHARACTER_ALREADY_ACTIVE" => Self::CharacterAlreadyActive,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progression {
    Retryable,
    Terminal,
    SecurityTerminal,
}

/// The public Gateway codes of N4-P §11.2. `SECURITY_TERMINAL` rows reach the client only as
/// [`GatewayErrorCode::AuthenticationRequired`] (§11.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayErrorCode {
    RequestMalformed,
    UnsupportedVersion,
    OfferUnsupported,
    AuthenticationRequired,
    AttemptConflict,
    CharacterConflict,
    RouteUnavailable,
    RateLimited,
    AttemptReconciliationRequired,
    GrantExpired,
    Unavailable,
}

/// (wire code, code, HTTP status, progression, public class), one row per N4-P §11.2 public code.
const GATEWAY_ERROR_ROWS: [(&str, GatewayErrorCode, u16, Progression, PublicClass); 11] = {
    use GatewayErrorCode as C;
    use Progression::{Retryable, SecurityTerminal, Terminal};
    use PublicClass::{
        AuthenticationRequired, ClientUpdateRequired, RetryLogin, SessionUnavailable,
        TemporarilyUnavailable,
    };
    [
        (
            "NATIVE_LOGIN_REQUEST_MALFORMED",
            C::RequestMalformed,
            400,
            Terminal,
            RetryLogin,
        ),
        (
            "NATIVE_LOGIN_UNSUPPORTED_VERSION",
            C::UnsupportedVersion,
            400,
            Terminal,
            ClientUpdateRequired,
        ),
        (
            "NATIVE_LOGIN_OFFER_UNSUPPORTED",
            C::OfferUnsupported,
            409,
            Terminal,
            ClientUpdateRequired,
        ),
        (
            "NATIVE_LOGIN_AUTHENTICATION_REQUIRED",
            C::AuthenticationRequired,
            401,
            SecurityTerminal,
            AuthenticationRequired,
        ),
        (
            "NATIVE_LOGIN_ATTEMPT_CONFLICT",
            C::AttemptConflict,
            409,
            Terminal,
            RetryLogin,
        ),
        (
            "NATIVE_LOGIN_CHARACTER_CONFLICT",
            C::CharacterConflict,
            409,
            Terminal,
            SessionUnavailable,
        ),
        (
            "NATIVE_LOGIN_ROUTE_UNAVAILABLE",
            C::RouteUnavailable,
            503,
            Retryable,
            TemporarilyUnavailable,
        ),
        (
            "NATIVE_LOGIN_RATE_LIMITED",
            C::RateLimited,
            429,
            Retryable,
            TemporarilyUnavailable,
        ),
        (
            "ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED",
            C::AttemptReconciliationRequired,
            503,
            Retryable,
            TemporarilyUnavailable,
        ),
        (
            "NATIVE_LOGIN_GRANT_EXPIRED",
            C::GrantExpired,
            409,
            Terminal,
            RetryLogin,
        ),
        (
            "NATIVE_LOGIN_UNAVAILABLE",
            C::Unavailable,
            503,
            Retryable,
            TemporarilyUnavailable,
        ),
    ]
};

impl GatewayErrorCode {
    fn row(self) -> (&'static str, u16, Progression, PublicClass) {
        GATEWAY_ERROR_ROWS.iter().find(|row| row.1 == self).map_or(
            (
                "",
                0,
                Progression::Terminal,
                PublicClass::TemporarilyUnavailable,
            ),
            |&(wire, _, status, progression, class)| (wire, status, progression, class),
        )
    }

    #[must_use]
    pub fn wire_code(self) -> &'static str {
        self.row().0
    }

    #[must_use]
    pub fn http_status(self) -> u16 {
        self.row().1
    }

    #[must_use]
    pub fn progression(self) -> Progression {
        self.row().2
    }

    #[must_use]
    pub fn public_class(self) -> PublicClass {
        self.row().3
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeLoginError {
    /// A transport, configuration or payload failure; fails closed.
    Platform(PlatformClientError),
    /// The OAuth token exchange was refused.
    TokenRefused,
    /// Platform refused to issue a native ticket for the bearer.
    TicketRefused(PublicClass),
    /// The Gateway answered one of its N4-P §11 codes and no retry applies or remains.
    Gateway(GatewayErrorCode),
    /// The ticket's lifetime or its Gateway request budget is spent; a fresh login is needed.
    TicketSpent,
}

impl NativeLoginError {
    /// The public class the player sees. Anything not classified by a contract row is generic
    /// `TEMPORARILY_UNAVAILABLE`.
    #[must_use]
    pub fn public_class(self) -> PublicClass {
        match self {
            Self::Platform(_) => PublicClass::TemporarilyUnavailable,
            Self::TokenRefused => PublicClass::RetryLogin,
            Self::TicketRefused(class) => class,
            Self::Gateway(code) => code.public_class(),
            Self::TicketSpent => PublicClass::RetryLogin,
        }
    }
}

impl Display for NativeLoginError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Platform(error) => Display::fmt(error, formatter),
            Self::TokenRefused => formatter.write_str("OAuth token exchange was refused"),
            Self::TicketRefused(class) => write!(formatter, "native ticket refused ({class:?})"),
            Self::Gateway(code) => write!(formatter, "Gateway login refused: {}", code.wire_code()),
            Self::TicketSpent => {
                formatter.write_str("native ticket lifetime or request budget spent")
            }
        }
    }
}

impl std::error::Error for NativeLoginError {}

impl From<PlatformClientError> for NativeLoginError {
    fn from(error: PlatformClientError) -> Self {
        Self::Platform(error)
    }
}

/// Fills `buffer` from the operating system CSPRNG (via the rustls crypto provider).
pub fn secure_random(buffer: &mut [u8]) -> Result<(), PlatformClientError> {
    rustls::crypto::aws_lc_rs::default_provider()
        .secure_random
        .fill(buffer)
        .map_err(|_error| PlatformClientError::ClientCreation)
}

/// One logical login attempt's `attempt_ref`: a canonical lowercase UUIDv7, generated once and
/// reused byte-for-byte on every retry of that attempt (N4-P §6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptRef(String);

impl AttemptRef {
    pub fn generate() -> Result<Self, PlatformClientError> {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_error| PlatformClientError::ClientCreation)?
            .as_millis();
        let mut bytes = [0_u8; 16];
        secure_random(&mut bytes[6..])?;
        let timestamp = u64::try_from(millis & 0xffff_ffff_ffff)
            .map_err(|_error| PlatformClientError::ClientCreation)?
            .to_be_bytes();
        bytes[..6].copy_from_slice(&timestamp[2..]);
        bytes[6] = 0x70 | (bytes[6] & 0x0f);
        bytes[8] = 0x80 | (bytes[8] & 0x3f);
        Ok(Self(format_uuid(&bytes)))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Parses a canonical lowercase RFC 9562 UUIDv7 (version 7, RFC variant) into its bytes.
#[must_use]
pub fn parse_uuid_v7(value: &str) -> Option<[u8; 16]> {
    let text = value.as_bytes();
    if text.len() != 36 {
        return None;
    }
    let mut bytes = [0_u8; 16];
    let mut index = 0;
    let mut nibble = 0;
    for (position, &character) in text.iter().enumerate() {
        if matches!(position, 8 | 13 | 18 | 23) {
            if character != b'-' {
                return None;
            }
            continue;
        }
        let digit = match character {
            b'0'..=b'9' => character - b'0',
            b'a'..=b'f' => character - b'a' + 10,
            _ => return None,
        };
        if nibble == 0 {
            bytes[index] = digit << 4;
            nibble = 1;
        } else {
            bytes[index] |= digit;
            index += 1;
            nibble = 0;
        }
    }
    (bytes[6] >> 4 == 7 && bytes[8] >> 6 == 0b10).then_some(bytes)
}

fn format_uuid(bytes: &[u8; 16]) -> String {
    let mut text = String::with_capacity(36);
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 4 | 6 | 8 | 10) {
            text.push('-');
        }
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

/// A Platform OAuth access token for scope `game:ticket`. Consumed by ticket issuance, which
/// revokes it on the Platform side.
#[derive(Debug)]
pub struct AccessToken(SecretText);

/// A one-time native Game Login Ticket, the instant after which the client stops using it, and
/// the Gateway requests already sent with it (one budget for the ticket and its `attempt_ref`).
#[derive(Debug)]
pub struct NativeTicket {
    secret: SecretText,
    usable_until: Instant,
    requests_sent: AtomicU32,
}

impl NativeTicket {
    #[must_use]
    pub const fn usable_until(&self) -> Instant {
        self.usable_until
    }
}

/// The Gateway login request's caller-chosen members (N4-P §3.1).
#[derive(Debug, Clone)]
pub struct GatewayLogin<'a> {
    pub attempt_ref: &'a AttemptRef,
    /// Canonical lowercase UUIDv7 (mode 33a: from configuration).
    pub character_id: &'a str,
    /// 1..64 visible ASCII; diagnostics only.
    pub client_build: &'a str,
}

/// The routed endpoint of a successful login (N4-P §3.2). The client connects here and nowhere
/// else, verifies `tls_server_name`, and requires ALPN [`GAME_ALPN`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameEndpoint {
    pub host: String,
    pub port: u16,
    pub tls_server_name: String,
}

/// A successful Gateway login: the route and the pre-admission grant.
#[derive(Debug)]
pub struct NativeLoginGrant {
    pub world_id: String,
    pub channel_id: String,
    pub endpoint: GameEndpoint,
    grant: SecretText,
    usable_until: Instant,
}

impl NativeLoginGrant {
    /// The grant bytes for `Admission::admission_material`, exactly as received.
    #[must_use]
    pub fn grant_bytes(&self) -> &[u8] {
        self.grant.expose().as_bytes()
    }

    /// When the grant's reported validity ends, measured from its receipt; used only to stop
    /// retrying, never to extend validity.
    #[must_use]
    pub const fn usable_until(&self) -> Instant {
        self.usable_until
    }
}

/// Retry pacing; [`RetryPacing::CONTRACT`] is the production value, tests shorten it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPacing {
    /// Backoff before retry `n` (1-based) is `unit * 2^(n-1)`.
    pub unit: Duration,
    /// A `Retry-After` longer than this is not waited for.
    pub max_retry_after: Duration,
}

impl RetryPacing {
    pub const CONTRACT: Self = Self {
        unit: Duration::from_secs(1),
        max_retry_after: MAX_RETRY_AFTER,
    };
}

pub struct NativeLoginClient {
    client: Client,
    platform: Url,
    gateway: Url,
    oauth_client_id: String,
    pacing: RetryPacing,
}

impl NativeLoginClient {
    pub fn new(
        platform: &PlatformClientConfig,
        gateway: &PlatformClientConfig,
        oauth_client_id: &str,
    ) -> Result<Self, PlatformClientError> {
        if oauth_client_id.is_empty()
            || oauth_client_id.len() > 128
            || !oauth_client_id.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(PlatformClientError::ClientCreation);
        }
        let client = Client::builder()
            .redirect(Policy::none())
            .no_proxy()
            .connect_timeout(platform.connect_timeout)
            .timeout(platform.request_timeout)
            .build()
            .map_err(|_error| PlatformClientError::ClientCreation)?;
        Ok(Self {
            client,
            platform: platform.base_url.clone(),
            gateway: gateway.base_url.clone(),
            oauth_client_id: oauth_client_id.to_owned(),
            pacing: RetryPacing::CONTRACT,
        })
    }

    #[must_use]
    pub const fn with_pacing(mut self, pacing: RetryPacing) -> Self {
        self.pacing = pacing;
        self
    }

    /// The system-browser authorization URL: code flow, PKCE S256, one exact `state`, scope
    /// `game:ticket`, loopback `redirect_uri`.
    pub fn authorization_url(
        &self,
        redirect_uri: &str,
        state: &str,
        code_challenge: &str,
    ) -> Result<Url, PlatformClientError> {
        let mut url = self
            .platform
            .join("oauth/authorize")
            .map_err(|_error| PlatformClientError::InvalidBaseUrl)?;
        url.query_pairs_mut()
            .append_pair("client_id", &self.oauth_client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", NATIVE_LOGIN_SCOPE)
            .append_pair("state", state)
            .append_pair("code_challenge", code_challenge)
            .append_pair("code_challenge_method", "S256");
        Ok(url)
    }

    /// Exchanges the authorization code for a bearer access token.
    pub async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
        cancellation: CancellationToken,
    ) -> Result<AccessToken, NativeLoginError> {
        let endpoint = self
            .platform
            .join("oauth/token")
            .map_err(|_error| PlatformClientError::InvalidBaseUrl)?;
        let request = async {
            let response = self
                .client
                .post(endpoint)
                .form(&[
                    ("grant_type", "authorization_code"),
                    ("client_id", self.oauth_client_id.as_str()),
                    ("redirect_uri", redirect_uri),
                    ("code_verifier", code_verifier),
                    ("code", code),
                ])
                .send()
                .await
                .map_err(|_error| PlatformClientError::Request)?;
            if !response.status().is_success() {
                return Err(NativeLoginError::TokenRefused);
            }
            let body = read_bounded(response, MAX_OAUTH_RESPONSE_BYTES).await?;
            // OAuth token responses carry further members (refresh token, expiry); only these two
            // are read, and the rest is dropped with the body.
            #[derive(Deserialize)]
            struct TokenResponse {
                token_type: String,
                access_token: String,
            }
            let token: TokenResponse = serde_json::from_slice(&body)
                .map_err(|_error| PlatformClientError::InvalidPayload)?;
            if !token.token_type.eq_ignore_ascii_case("bearer")
                || !is_visible_ascii(&token.access_token, MAX_ACCESS_TOKEN_BYTES)
            {
                return Err(PlatformClientError::InvalidPayload.into());
            }
            Ok(AccessToken(SecretText(token.access_token)))
        };
        run(cancellation, request).await
    }

    /// `POST /v1/game-auth/tickets` with the bearer: one native ticket. The access token is
    /// consumed; Platform revokes it.
    pub async fn issue_native_ticket(
        &self,
        access_token: AccessToken,
        cancellation: CancellationToken,
    ) -> Result<NativeTicket, NativeLoginError> {
        let endpoint = self
            .platform
            .join("v1/game-auth/tickets")
            .map_err(|_error| PlatformClientError::InvalidBaseUrl)?;
        let request = async {
            let response = self
                .client
                .post(endpoint)
                .bearer_auth(access_token.0.expose())
                .json(&serde_json::json!({ "protocol_version": 1 }))
                .send()
                .await
                .map_err(|_error| PlatformClientError::Request)?;
            let received = Instant::now();
            match response.status() {
                status if status.is_success() => {}
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    return Err(NativeLoginError::TicketRefused(
                        PublicClass::AuthenticationRequired,
                    ));
                }
                _ => {
                    return Err(NativeLoginError::TicketRefused(
                        PublicClass::TemporarilyUnavailable,
                    ));
                }
            }
            let body = read_bounded(response, MAX_TICKET_RESPONSE_BYTES).await?;
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct TicketResponse {
                protocol_version: u64,
                ticket: String,
                expires_in: u64,
            }
            let ticket: TicketResponse = serde_json::from_slice(&body)
                .map_err(|_error| PlatformClientError::InvalidPayload)?;
            if ticket.protocol_version != 1
                || ticket.expires_in == 0
                || !is_visible_ascii(&ticket.ticket, MAX_TICKET_BYTES)
            {
                return Err(PlatformClientError::InvalidPayload.into());
            }
            let lifetime = Duration::from_secs(ticket.expires_in).min(MAX_TICKET_LIFETIME);
            Ok(NativeTicket {
                secret: SecretText(ticket.ticket),
                usable_until: received + lifetime,
                requests_sent: AtomicU32::new(0),
            })
        };
        run(cancellation, request).await
    }

    /// Gateway `POST /v1/login` (protocol 2). Retries the byte-identical request (same
    /// `attempt_ref`, same ticket) on a `RETRYABLE` row or a lost response. Every call with the
    /// same ticket shares one budget of [`MAX_LOGIN_REQUESTS`] requests, and no request is sent
    /// past the ticket's lifetime.
    pub async fn gateway_login(
        &self,
        ticket: &NativeTicket,
        login: &GatewayLogin<'_>,
        cancellation: CancellationToken,
    ) -> Result<NativeLoginGrant, NativeLoginError> {
        let endpoint = self
            .gateway
            .join("v1/login")
            .map_err(|_error| PlatformClientError::InvalidBaseUrl)?;
        if parse_uuid_v7(login.character_id).is_none() || !is_visible_ascii(login.client_build, 64)
        {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        let body = serde_json::to_vec(&serde_json::json!({
            "protocol_version": GATEWAY_PROTOCOL_VERSION,
            "game_login_ticket": ticket.secret.expose(),
            "attempt_ref": login.attempt_ref.as_str(),
            "character_id": login.character_id,
            "channel_id": null,
            "offer": {
                "client_build": login.client_build,
                "client_platform": "windows",
                "transports": [
                    { "protocol_major": 1, "transport_profile": 1, "alpn": GAME_ALPN }
                ],
            },
        }))
        .map_err(|_error| PlatformClientError::InvalidPayload)?;
        if body.len() > MAX_LOGIN_REQUEST_BYTES {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        let attempts = async {
            loop {
                if Instant::now() >= ticket.usable_until {
                    return Err(NativeLoginError::TicketSpent);
                }
                let request_number = ticket.requests_sent.fetch_add(1, Ordering::Relaxed) + 1;
                if request_number > MAX_LOGIN_REQUESTS {
                    return Err(NativeLoginError::TicketSpent);
                }
                let (error, retry_after) =
                    match self.login_once(&endpoint, &body, login.attempt_ref).await {
                        Ok(grant) => return Ok(grant),
                        Err(failure) => failure,
                    };
                let retryable = match error {
                    NativeLoginError::Gateway(code) => code.progression() == Progression::Retryable,
                    // A lost or cut response: the identical request returns the identical
                    // grant (N4-P §6).
                    NativeLoginError::Platform(PlatformClientError::Request) => true,
                    _ => false,
                };
                if !retryable || request_number >= MAX_LOGIN_REQUESTS {
                    return Err(error);
                }
                let backoff = retry_after
                    .unwrap_or_else(|| self.pacing.unit.saturating_mul(1 << (request_number - 1)));
                if backoff > self.pacing.max_retry_after
                    || Instant::now() + backoff >= ticket.usable_until
                {
                    return Err(error);
                }
                tokio::time::sleep(backoff).await;
            }
        };
        run(cancellation, attempts).await
    }

    /// One Gateway request. On failure, returns the error and the server's `Retry-After`.
    async fn login_once(
        &self,
        endpoint: &Url,
        body: &[u8],
        attempt_ref: &AttemptRef,
    ) -> Result<NativeLoginGrant, (NativeLoginError, Option<Duration>)> {
        let response = self
            .client
            .post(endpoint.clone())
            .header(CONTENT_TYPE, "application/json")
            .body(body.to_vec())
            .send()
            .await
            .map_err(|_error| (PlatformClientError::Request.into(), None))?;
        let received = Instant::now();
        let status = response.status();
        let retry_after = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs);
        let body = read_bounded(response, MAX_LOGIN_RESPONSE_BYTES)
            .await
            .map_err(|error| (error.into(), None))?;
        if status == StatusCode::OK {
            return parse_login_success(&body, attempt_ref, received)
                .map_err(|error| (error.into(), None));
        }
        let code = parse_login_error(status.as_u16(), &body, attempt_ref)
            .map_err(|error| (error.into(), None))?;
        Err((NativeLoginError::Gateway(code), retry_after))
    }
}

async fn run<T, F>(cancellation: CancellationToken, future: F) -> Result<T, NativeLoginError>
where
    F: Future<Output = Result<T, NativeLoginError>>,
{
    match cancellable(cancellation, future).await {
        Cancellable::Completed(result) => result,
        Cancellable::Cancelled => Err(PlatformClientError::Cancelled.into()),
    }
}

pub(crate) async fn read_bounded(
    mut response: Response,
    max_bytes: usize,
) -> Result<Vec<u8>, PlatformClientError> {
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes as u64)
    {
        return Err(PlatformClientError::ResponseTooLarge);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_error| PlatformClientError::Request)?
    {
        let next_len = body
            .len()
            .checked_add(chunk.len())
            .ok_or(PlatformClientError::ResponseTooLarge)?;
        if next_len > max_bytes {
            return Err(PlatformClientError::ResponseTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// Exactly `{"protocol_version":2,"attempt_ref",...}` of N4-P §3.2. Unknown, duplicate, missing
/// and `null` members are rejected by the derived decoders.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginSuccess {
    protocol_version: u64,
    attempt_ref: String,
    world_id: String,
    channel_id: String,
    endpoint: LoginEndpoint,
    grant: LoginGrant,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginEndpoint {
    host: String,
    port: u16,
    tls_server_name: String,
    alpn: String,
    protocol_major: u64,
    transport_profile: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginGrant {
    profile: String,
    token: String,
    valid_for_seconds: u64,
}

/// Decodes and validates a `200` Gateway body.
pub fn parse_login_success(
    body: &[u8],
    attempt_ref: &AttemptRef,
    received: Instant,
) -> Result<NativeLoginGrant, PlatformClientError> {
    if body.len() > MAX_LOGIN_RESPONSE_BYTES {
        return Err(PlatformClientError::ResponseTooLarge);
    }
    let success: LoginSuccess =
        serde_json::from_slice(body).map_err(|_error| PlatformClientError::InvalidPayload)?;
    let endpoint = success.endpoint;
    let grant = success.grant;
    let valid = success.protocol_version == GATEWAY_PROTOCOL_VERSION
        && success.attempt_ref == attempt_ref.as_str()
        && parse_uuid_v7(&success.world_id).is_some()
        && parse_uuid_v7(&success.channel_id).is_some()
        && is_host(&endpoint.host)
        && endpoint.port != 0
        && is_host(&endpoint.tls_server_name)
        && endpoint.alpn == GAME_ALPN
        && endpoint.protocol_major == 1
        && endpoint.transport_profile == 1
        && grant.profile == GRANT_PROFILE
        && is_jws_compact(&grant.token)
        && grant.valid_for_seconds >= 1;
    if !valid {
        return Err(PlatformClientError::InvalidPayload);
    }
    Ok(NativeLoginGrant {
        world_id: success.world_id,
        channel_id: success.channel_id,
        endpoint: GameEndpoint {
            host: endpoint.host,
            port: endpoint.port,
            tls_server_name: endpoint.tls_server_name,
        },
        grant: SecretText(grant.token),
        usable_until: received + Duration::from_secs(grant.valid_for_seconds.min(3600)),
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginErrorBody {
    protocol_version: u64,
    error: LoginErrorDetail,
    #[serde(deserialize_with = "required_nullable")]
    attempt_ref: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginErrorDetail {
    code: String,
    public_class: String,
}

/// A present member that may be `null` (a missing one still fails: no `default`).
fn required_nullable<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}

/// Decodes a non-`200` Gateway body (N4-P §11.1). The code, its HTTP status and its public class
/// must agree with one §11.2 row, and an echoed `attempt_ref` must be ours; anything else fails
/// closed.
pub fn parse_login_error(
    status: u16,
    body: &[u8],
    attempt_ref: &AttemptRef,
) -> Result<GatewayErrorCode, PlatformClientError> {
    let error: LoginErrorBody =
        serde_json::from_slice(body).map_err(|_error| PlatformClientError::InvalidPayload)?;
    if error.protocol_version != GATEWAY_PROTOCOL_VERSION
        || error
            .attempt_ref
            .is_some_and(|echo| echo != attempt_ref.as_str())
    {
        return Err(PlatformClientError::InvalidPayload);
    }
    let class =
        PublicClass::parse(&error.error.public_class).ok_or(PlatformClientError::InvalidPayload)?;
    GATEWAY_ERROR_ROWS
        .iter()
        .find(|row| row.0 == error.error.code)
        .filter(|row| row.2 == status && row.4 == class)
        .map(|row| row.1)
        .ok_or(PlatformClientError::InvalidPayload)
}

fn is_visible_ascii(value: &str, max_bytes: usize) -> bool {
    !value.is_empty() && value.len() <= max_bytes && value.bytes().all(|b| b.is_ascii_graphic())
}

/// A DNS name (1..253 bytes, LDH labels of 1..63) or an IP literal.
fn is_host(value: &str) -> bool {
    if value.parse::<IpAddr>().is_ok() {
        return true;
    }
    !value.is_empty()
        && value.len() <= 253
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

/// JWS compact serialization: three base64url segments, at most 4096 bytes.
fn is_jws_compact(value: &str) -> bool {
    value.len() <= MAX_GRANT_TOKEN_BYTES
        && value.split('.').count() == 3
        && value.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        })
}

#[cfg(test)]
mod tests;
