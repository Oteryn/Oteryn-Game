//! Native login against Platform and the Gateway (N4-P contract at Platform `f880cd74`,
//! ARCH-LOGIN-FIRST-0 §2.4): OAuth authorization code + PKCE S256 URL and token exchange, native
//! ticket issuance, and the Gateway `POST /v1/login` protocol-2 branch.
//!
//! Tokens, tickets and grants live only in [`SecretText`] values: in memory, redacted in `Debug`
//! and `Display`, never logged. This module decodes its own typed Gateway response, so it does not
//! pass through the directory's forbidden-field filter; nothing else may.

mod account_services;
pub mod bazaar;
mod challenge;
pub mod coin_purchase;
mod gateway_refusal;
mod gateway_request;
pub use challenge::{ChallengeProof, LoginChallenge, TokenExchange};
mod owner_characters;
pub mod premium;
mod ticket;
use gateway_refusal::maintenance_deadline;
#[cfg(test)]
use gateway_refusal::{GATEWAY_ERROR_ROWS, maintenance_delay};
pub use gateway_refusal::{
    GatewayErrorCode, GatewayRefusal, Progression, PublicClass, UpdatePointer,
    parse_gateway_refusal, parse_login_error,
};
pub use owner_characters::{CharacterAvailability, GatewayLogin, OwnerCharacter};

use crate::updates::{UpdateChannel, valid_release_sequence};
use crate::{PlatformClientConfig, PlatformClientError};
use gateway_request::GatewayOffer;
use oteryn_foundation::{Cancellable, CancellationToken, cancellable};
use reqwest::header::{CONTENT_TYPE, RETRY_AFTER};
use reqwest::{Client, Response, StatusCode, Url, redirect::Policy};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug, Display, Formatter};
use std::net::IpAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The normal gameplay OAuth scope (N4-P §2 step 1); account services opt in separately.
pub const NATIVE_LOGIN_SCOPE: &str = "game:ticket";
/// The Gateway native branch version (N4-P §3.1).
pub const GATEWAY_PROTOCOL_VERSION: u64 = 2;
pub(crate) const GATEWAY_LOGIN_PATH: &str = "v1/login";
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

/// The Downloads identity compiled into an installed client build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ReleaseIdentity {
    update_channel: UpdateChannel,
    release_id: &'static str,
    release_sequence: u64,
}

impl ReleaseIdentity {
    #[must_use]
    pub const fn new(
        update_channel: UpdateChannel,
        release_id: &'static str,
        release_sequence: u64,
    ) -> Option<Self> {
        let bytes = release_id.as_bytes();
        if bytes.is_empty() || bytes.len() > 96 || !valid_release_sequence(release_sequence) {
            return None;
        }
        let mut i = 0;
        while i < bytes.len() {
            let byte = bytes[i];
            if !(byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')) {
                return None;
            }
            i += 1;
        }
        Some(Self {
            update_channel,
            release_id,
            release_sequence,
        })
    }

    #[must_use]
    pub const fn update_channel(self) -> UpdateChannel {
        self.update_channel
    }

    /// A signed current target must differ from the identity that was refused.
    pub(crate) fn matches_target(self, release_id: &str, sequence: u64) -> bool {
        self.release_sequence == sequence || self.release_id == release_id
    }
}

/// A signed device proof for §19.4 recovery. It serializes only in the request body.
#[derive(Serialize)]
pub struct RecoveryDeviceProof<'a> {
    pub kid: &'a str,
    pub sig: &'a str,
}

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

/// Exact, bounded device-route refusal. Only explicit protocol/site errors are retained;
/// arbitrary server response text and credentials can never enter diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceRefusal {
    pub status: u16,
    pub error: &'static str,
}
impl DeviceRefusal {
    fn decode(status: u16, bytes: &[u8]) -> Result<Self, PlatformClientError> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Body<'a> {
            #[serde(borrow)]
            error: &'a str,
        }
        // a85b5da4 still wraps native routes in web/src/surface.rs:244-268;
        // web/src/ratelimit.rs:356-389 also renders API 503 envelopes.
        // With explicit JSON/English negotiation it emits this exact envelope.
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct SiteBody<'a> {
            status: u16,
            #[serde(borrow)]
            error: &'a str,
        }
        if let Ok(body) = serde_json::from_slice::<SiteBody<'_>>(bytes) {
            let error = match (status, body.status, body.error) {
                (400, 400, "Invalid request") => "Invalid request",
                (503, 503, "Temporarily unavailable") => "Temporarily unavailable",
                _ => return Err(PlatformClientError::InvalidPayload),
            };
            return Ok(Self { status, error });
        }
        let body: Body<'_> =
            serde_json::from_slice(bytes).map_err(|_| PlatformClientError::InvalidPayload)?;
        let error = match body.error {
            "invalid_request" => "invalid_request",
            "device_authorization_unavailable" => "device_authorization_unavailable",
            "too_many_requests" => "too_many_requests",
            _ => return Err(PlatformClientError::InvalidPayload),
        };
        Ok(Self { status, error })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeLoginError {
    /// A transport, configuration or payload failure; fails closed.
    Platform(PlatformClientError),
    /// The OAuth token exchange was refused.
    TokenRefused,
    /// Device HTTP refusal with the exact contract status and error value.
    DeviceRefused(DeviceRefusal),
    /// Platform refused to issue a native ticket for the bearer.
    TicketRefused(PublicClass),
    /// The Gateway answered one of its N4-P §11 codes and no retry applies or remains.
    Gateway(GatewayErrorCode),
    /// Terminal for this running build, regardless of the remaining ticket budget.
    UpdateRequired(UpdatePointer),
    /// Retry with a fresh sign-in once this monotonic deadline passes.
    Maintenance { retry_at: Instant },
    /// The ticket's lifetime or its Gateway request budget is spent; a fresh login is needed.
    TicketSpent,
}

impl NativeLoginError {
    /// The public class the player sees. Anything not classified by a contract row is generic
    /// `TEMPORARILY_UNAVAILABLE`.
    #[must_use]
    pub fn public_class(self) -> PublicClass {
        match self {
            Self::Platform(_) | Self::Maintenance { .. } => PublicClass::TemporarilyUnavailable,
            Self::UpdateRequired(_) => PublicClass::ClientUpdateRequired,
            Self::TokenRefused | Self::DeviceRefused(_) => PublicClass::RetryLogin,
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
            Self::DeviceRefused(refusal) => write!(
                formatter,
                "device request refused ({} {})",
                refusal.status, refusal.error
            ),
            Self::TicketRefused(class) => write!(formatter, "native ticket refused ({class:?})"),
            Self::Gateway(code) => write!(formatter, "Gateway login refused: {}", code.wire_code()),
            Self::UpdateRequired(_) => formatter.write_str("NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED"),
            Self::Maintenance { .. } => formatter.write_str("NATIVE_LOGIN_MAINTENANCE"),
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

impl AccessToken {
    /// The bearer value for a Platform request; never logged.
    pub(crate) fn expose(&self) -> &str {
        self.0.expose()
    }
}

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

/// Retry pacing; tests advance waits without changing limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPacing {
    /// Backoff before retry `n` (1-based) is `unit * 2^(n-1)`.
    pub unit: Duration,
    /// Maximum accepted `Retry-After`.
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
    platform_web: Url,
    gateway: Url,
    oauth_client_id: String,
    pacing: RetryPacing,
    release: Option<ReleaseIdentity>,
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
            platform: platform.api_base_url.clone(),
            platform_web: platform.base_url.clone(),
            gateway: gateway.base_url.clone(),
            oauth_client_id: oauth_client_id.to_owned(),
            pacing: RetryPacing::CONTRACT,
            release: None,
        })
    }

    #[must_use]
    pub const fn with_pacing(mut self, pacing: RetryPacing) -> Self {
        self.pacing = pacing;
        self
    }

    /// Enable only for environments whose Gateway and issuer accept `offer.release` (§3.3).
    #[must_use]
    pub const fn with_release(mut self, release: Option<ReleaseIdentity>) -> Self {
        self.release = release;
        self
    }

    /// §6.1 digest of the offer shared by protocol 2, protocol 3 and recovery.
    pub fn offer_digest(&self, client_build: &str) -> Result<String, PlatformClientError> {
        GatewayOffer::new(client_build, self.release)?.digest()
    }

    /// Encode a protocol 2 or 3 login request using this client's configured offer.
    pub fn login_request(
        &self,
        ticket: &NativeTicket,
        login: &GatewayLogin<'_>,
        protocol_version: u64,
    ) -> Result<Vec<u8>, PlatformClientError> {
        GatewayOffer::new(login.client_build, self.release)?.login(
            ticket.secret.expose(),
            login,
            protocol_version,
        )
    }

    /// Encode the §19.4 request. An absent proof is allowed only on a testing Gateway
    /// with device binding disabled.
    pub fn recovery_request(
        &self,
        recovery_handle: &str,
        attempt_ref: &AttemptRef,
        character_id: &str,
        client_build: &str,
        device_proof: Option<&RecoveryDeviceProof<'_>>,
    ) -> Result<Vec<u8>, PlatformClientError> {
        GatewayOffer::new(client_build, self.release)?.recover(
            recovery_handle,
            attempt_ref,
            character_id,
            device_proof,
        )
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
            .platform_web
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
        match self
            .exchange_code_challenged(code, code_verifier, redirect_uri, None, cancellation)
            .await?
        {
            TokenExchange::Token(token) => Ok(token),
            TokenExchange::Challenge(_) => Err(NativeLoginError::TokenRefused),
        }
    }

    /// `POST /api/v1/game-auth/tickets` with the bearer: one native ticket. The access token is
    /// consumed; Platform revokes it.
    pub async fn issue_native_ticket(
        &self,
        access_token: AccessToken,
        cancellation: CancellationToken,
    ) -> Result<NativeTicket, NativeLoginError> {
        ticket::issue(self, access_token, cancellation).await
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
            .join(GATEWAY_LOGIN_PATH)
            .map_err(|_error| PlatformClientError::InvalidBaseUrl)?;
        let body = self.login_request(ticket, login, GATEWAY_PROTOCOL_VERSION)?;
        let mut span = oteryn_error_codes::Span::start(
            oteryn_error_codes::Level::Info,
            "platform",
            "platform_call",
            None,
        )
        .slow_after(crate::PLATFORM_SLOW_OPERATION);
        span.field("endpoint", "gateway_login");
        let started = Instant::now();
        let mut attempts_made = 0_u64;
        let attempts = async {
            loop {
                if Instant::now() >= ticket.usable_until {
                    return Err(NativeLoginError::TicketSpent);
                }
                let request_number = ticket.requests_sent.fetch_add(1, Ordering::Relaxed) + 1;
                if request_number > MAX_LOGIN_REQUESTS {
                    return Err(NativeLoginError::TicketSpent);
                }
                attempts_made = attempts_made.saturating_add(1);
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
                    || backoff
                        >= ticket
                            .usable_until
                            .saturating_duration_since(Instant::now())
                {
                    return Err(error);
                }
                #[cfg(test)]
                let backoff = tests::delay(backoff);
                tokio::time::sleep(backoff).await;
            }
        };
        let result = run(cancellation, attempts).await;
        oteryn_error_codes::metrics::Registry::global()
            .histogram("platform_call_ms", &[("endpoint", "gateway_login")])
            .observe(u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX));
        span.field("attempts", attempts_made);
        match result {
            Ok(grant) => {
                span.set_outcome("ok");
                span.field("status", 200_u64);
                Ok(grant)
            }
            Err(error) => {
                span.set_outcome("failed");
                match &error {
                    NativeLoginError::UpdateRequired(_) => {
                        span.field(
                            "refusal",
                            GatewayErrorCode::ClientUpdateRequired.wire_code(),
                        );
                        span.field("status", 409_u64);
                    }
                    NativeLoginError::Maintenance { .. } => {
                        span.field("refusal", GatewayErrorCode::Maintenance.wire_code());
                        span.field("status", 503_u64);
                    }
                    NativeLoginError::Platform(platform_error) => {
                        span.set_code(crate::platform_error_code(*platform_error));
                    }
                    NativeLoginError::Gateway(code)
                        if code.progression() != Progression::Retryable =>
                    {
                        span.set_code(oteryn_error_codes::ErrorCode::new(
                            5006,
                            "PLATFORM_RESPONSE_INVALID",
                            oteryn_error_codes::Category::InternalUnavailable,
                            oteryn_error_codes::Progression::Terminal,
                        ));
                    }
                    _ => span.set_code(oteryn_error_codes::ErrorCode::new(
                        5007,
                        "PLATFORM_UNAVAILABLE",
                        oteryn_error_codes::Category::DependencyUnavailable,
                        oteryn_error_codes::Progression::Retryable,
                    )),
                }
                Err(error)
            }
        }
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
        let received_wall = SystemTime::now();
        let received = Instant::now();
        let status = response.status();
        let retry_after = response.headers().get(RETRY_AFTER).cloned();
        let retry_after = retry_after.as_ref().and_then(|value| value.to_str().ok());
        let body = read_bounded(response, MAX_LOGIN_RESPONSE_BYTES)
            .await
            .map_err(|error| (error.into(), None))?;
        if status == StatusCode::OK {
            return parse_login_success(&body, attempt_ref, received)
                .map_err(|error| (error.into(), None));
        }
        let refusal = parse_gateway_refusal(
            GATEWAY_PROTOCOL_VERSION,
            status.as_u16(),
            &body,
            attempt_ref,
        )
        .map_err(|error| (error.into(), None))?;
        match (refusal, self.release) {
            (GatewayRefusal::UpdateRequired(update), Some(release))
                if release.update_channel == update.update_channel =>
            {
                Err((NativeLoginError::UpdateRequired(update), None))
            }
            (GatewayRefusal::Code(GatewayErrorCode::Maintenance), Some(_)) => Err((
                NativeLoginError::Maintenance {
                    retry_at: maintenance_deadline(received, retry_after, received_wall),
                },
                None,
            )),
            (
                GatewayRefusal::UpdateRequired(_)
                | GatewayRefusal::Code(GatewayErrorCode::Maintenance),
                _,
            ) => Err((PlatformClientError::InvalidPayload.into(), None)),
            (GatewayRefusal::Code(code), _) => Err((
                NativeLoginError::Gateway(code),
                retry_after
                    .and_then(|value| value.parse::<u64>().ok())
                    .map(Duration::from_secs),
            )),
        }
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
