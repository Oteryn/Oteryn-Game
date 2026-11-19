//! Exact Gateway refusal shapes and the next authority for build gate responses.
use super::{AttemptRef, GATEWAY_PROTOCOL_VERSION};
use crate::PlatformClientError;
use crate::updates::{UpdateChannel, valid_release_sequence};
use reqwest::Url;
use serde::{Deserialize, Deserializer};
use std::time::{Duration, Instant, SystemTime};

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
    ClientUpdateRequired,
    Maintenance,
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
pub(super) const GATEWAY_ERROR_ROWS: [(&str, GatewayErrorCode, u16, Progression, PublicClass); 13] = {
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
            "NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED",
            C::ClientUpdateRequired,
            409,
            Terminal,
            ClientUpdateRequired,
        ),
        (
            "NATIVE_LOGIN_MAINTENANCE",
            C::Maintenance,
            503,
            Retryable,
            TemporarilyUnavailable,
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePointer {
    pub update_channel: UpdateChannel,
    #[serde(deserialize_with = "required_nullable")]
    pub minimum_release_sequence: Option<u64>,
    #[serde(deserialize_with = "required_nullable")]
    pub current_release_sequence: Option<u64>,
    pub manual_download_url: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginErrorBody {
    protocol_version: u64,
    #[serde(deserialize_with = "deserialize_object")]
    error: LoginErrorDetail,
    #[serde(deserialize_with = "required_nullable")]
    attempt_ref: Option<String>,
    #[serde(default, deserialize_with = "present_update")]
    update: Option<UpdatePointer>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginErrorDetail {
    code: String,
    public_class: String,
}

/// A present member that may be `null` (a missing one still fails: no `default`).
fn required_nullable<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(deserializer)
}

fn present_update<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<UpdatePointer>, D::Error> {
    deserialize_object(deserializer).map(Some)
}

#[derive(Debug, PartialEq, Eq)]
pub enum GatewayRefusal {
    Code(GatewayErrorCode),
    UpdateRequired(UpdatePointer),
}

impl GatewayRefusal {
    pub const fn code(&self) -> GatewayErrorCode {
        match self {
            Self::Code(code) => *code,
            Self::UpdateRequired(_) => GatewayErrorCode::ClientUpdateRequired,
        }
    }
}

/// Decodes a protocol 2 refusal, including validation of any required update pointer.
pub fn parse_login_error(
    status: u16,
    body: &[u8],
    attempt_ref: &AttemptRef,
) -> Result<GatewayErrorCode, PlatformClientError> {
    parse_gateway_refusal(GATEWAY_PROTOCOL_VERSION, status, body, attempt_ref)
        .map(|refusal| refusal.code())
}

/// Decodes a login or recovery refusal covered by §11.1 and minimum build gate §5.
/// The status, code, class, protocol version, echo and exact member set must agree.
pub fn parse_gateway_refusal(
    protocol_version: u64,
    status: u16,
    body: &[u8],
    attempt_ref: &AttemptRef,
) -> Result<GatewayRefusal, PlatformClientError> {
    let invalid = PlatformClientError::InvalidPayload;
    let JsonObject(error): JsonObject<LoginErrorBody> =
        serde_json::from_slice(body).map_err(|_| invalid)?;
    if !matches!(protocol_version, 2 | 3)
        || error.protocol_version != protocol_version
        || error
            .attempt_ref
            .is_some_and(|echo| echo != attempt_ref.as_str())
    {
        return Err(invalid);
    }
    let class = PublicClass::parse(&error.error.public_class).ok_or(invalid)?;
    let code = GATEWAY_ERROR_ROWS
        .iter()
        .find(|row| row.0 == error.error.code && row.2 == status && row.4 == class)
        .map(|row| row.1)
        .ok_or(invalid)?;

    match (code, error.update) {
        (GatewayErrorCode::ClientUpdateRequired, Some(update)) if valid_update(&update) => {
            Ok(GatewayRefusal::UpdateRequired(update))
        }
        (GatewayErrorCode::ClientUpdateRequired, _) | (_, Some(_)) => Err(invalid),
        (code, None) => Ok(GatewayRefusal::Code(code)),
    }
}

fn valid_update(update: &UpdatePointer) -> bool {
    let url = &update.manual_download_url;
    update
        .minimum_release_sequence
        .is_none_or(valid_release_sequence)
        && update
            .current_release_sequence
            .is_none_or(valid_release_sequence)
        && url
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
        && !url
            .chars()
            .any(|ch| ch == '\\' || ch.is_control() || ch.is_whitespace())
        && url
            .get(8..)
            .is_some_and(|authority| !authority.starts_with('/'))
        && Url::parse(url).is_ok_and(|url| {
            url.host().is_some() && url.username().is_empty() && url.password().is_none()
        })
}

// Derived struct visitors accept both maps and positional sequences.
#[derive(Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
struct JsonObject<T>(#[serde(deserialize_with = "deserialize_object")] T);

fn deserialize_object<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<T, D::Error> {
    struct ObjectVisitor<T>(std::marker::PhantomData<T>);

    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for ObjectVisitor<T> {
        type Value = T;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a JSON object")
        }

        fn visit_map<M: serde::de::MapAccess<'de>>(self, map: M) -> Result<T, M::Error> {
            T::deserialize(serde::de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_map(ObjectVisitor(std::marker::PhantomData))
}

pub(super) fn maintenance_delay(header: Option<&str>, now: SystemTime) -> Duration {
    let delay = header.map(str::trim).and_then(|value| {
        if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
            let seconds = value.parse::<u64>().unwrap_or(u64::MAX);
            Some(Duration::from_secs(seconds))
        } else {
            httpdate::parse_http_date(value)
                .ok()
                .map(|date| date.duration_since(now).unwrap_or_default())
        }
    });
    delay.unwrap_or_default().max(Duration::from_secs(1))
}

/// The automatic retry budget does not apply to the deadline for a fresh sign-in.
pub(super) fn maintenance_deadline(
    received: Instant,
    header: Option<&str>,
    wall: SystemTime,
) -> Instant {
    let delay = maintenance_delay(header, wall);
    if let Some(deadline) = received.checked_add(delay) {
        return deadline;
    }

    // Saturate an unrepresentable delay to the latest whole-second deadline.
    let (mut low, mut high) = (0, delay.as_secs());
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        if received.checked_add(Duration::from_secs(middle)).is_some() {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    received + Duration::from_secs(low)
}
