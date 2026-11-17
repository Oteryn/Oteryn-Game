//! Online purpose-separated observer admission. Only a successful authenticated
//! Platform consume response can construct the production proof. No reusable
//! password, CharacterId, actor or gameplay authority crosses this boundary.
use super::{Error, Settings, State, VerifiedAdmission};
use serde::Deserialize;
use std::time::Duration;

pub struct CastAccessClient {
    http: reqwest::Client,
    endpoint: reqwest::Url,
    node: [u8; 16],
    inflight: tokio::sync::Semaphore,
}
impl CastAccessClient {
    pub fn new(
        base: &str,
        node: [u8; 16],
        identity_pem: &[u8],
        ca_pem: &[u8],
    ) -> Result<Self, Error> {
        let base = reqwest::Url::parse(base).map_err(|_| Error::Invalid)?;
        if base.scheme() != "https"
            || base.host().is_none()
            || base.path() != "/"
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
            || node == [0; 16]
        {
            return Err(Error::Invalid);
        }
        let identity = reqwest::Identity::from_pem(identity_pem).map_err(|_| Error::Invalid)?;
        let roots = reqwest::Certificate::from_pem_bundle(ca_pem).map_err(|_| Error::Invalid)?;
        if roots.is_empty() {
            return Err(Error::Invalid);
        }
        let http = reqwest::Client::builder()
            .https_only(true)
            .min_tls_version(reqwest::tls::Version::TLS_1_3)
            .tls_certs_only(roots)
            .identity(identity)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|_| Error::Invalid)?;
        Ok(Self {
            http,
            endpoint: base
                .join("internal/v1/casts/node")
                .map_err(|_| Error::Invalid)?,
            node,
            inflight: tokio::sync::Semaphore::new(32),
        })
    }
    pub(super) async fn exchange(&self, body: serde_json::Value) -> Result<Vec<u8>, Error> {
        let _permit = self.inflight.try_acquire().map_err(|_| Error::Full)?;
        let mut response = self
            .http
            .post(self.endpoint.clone())
            .json(&body)
            .send()
            .await
            .map_err(|_| Error::Unavailable)?;
        if response.status() != 200 || response.content_length().is_some_and(|n| n > 1024) {
            return Err(Error::Unavailable);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Unavailable)? {
            if bytes.len() + chunk.len() > 1024 {
                return Err(Error::Unavailable);
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
    /// Clocks are sampled AFTER the HTTP response. Callers must not hold the
    /// world/tick owner while awaiting admission, renewal or release.
    pub async fn consume(
        &self,
        ticket: &[u8],
        clock: &dyn Fn() -> (u64, u64),
    ) -> Result<VerifiedAdmission, Error> {
        let ticket = std::str::from_utf8(ticket).map_err(|_| Error::Invalid)?;
        if ticket.len() != 69
            || !ticket.starts_with("ocv1.")
            || !ticket.as_bytes()[5..].iter().all(u8::is_ascii_hexdigit)
        {
            return Err(Error::Invalid);
        }
        let request_start = clock().1;
        let bytes = self
            .exchange(serde_json::json!({"operation":"consume", "ticket":ticket}))
            .await?;
        let (unix_seconds, monotonic_ms) = clock();
        let mut proof = decode_response(&bytes, self.node, unix_seconds, monotonic_ms)?;
        // Consume/renew grants exactly five seconds in Platform. Bound from
        // BEFORE IO, independent of UTC skew or response latency, so an old
        // observer never outlives the global quota reservation.
        proof.expires_ms = proof.expires_ms.min(request_start.saturating_add(4000));
        if proof.expires_ms <= monotonic_ms {
            return Err(Error::Unavailable);
        }
        Ok(proof)
    }
    pub async fn renew(
        &self,
        reservation: [u8; 16],
        clock: &dyn Fn() -> (u64, u64),
    ) -> Result<VerifiedAdmission, Error> {
        let request_start = clock().1;
        let bytes = self
            .exchange(serde_json::json!({"operation":"renew", "reservation":uuid(reservation)}))
            .await?;
        let (unix_seconds, monotonic_ms) = clock();
        let mut proof = decode_response(&bytes, self.node, unix_seconds, monotonic_ms)?;
        proof.expires_ms = proof.expires_ms.min(request_start.saturating_add(4000));
        if proof.expires_ms <= monotonic_ms {
            return Err(Error::Unavailable);
        }
        Ok(proof)
    }
    pub async fn release(&self, reservation: [u8; 16]) -> Result<(), Error> {
        self.exchange(serde_json::json!({"operation":"release", "reservation":uuid(reservation)}))
            .await?;
        Ok(())
    }
    pub async fn announce(
        &self,
        owner: [u8; 16],
        state: State,
        settings: Settings,
    ) -> Result<(), Error> {
        self.exchange(serde_json::json!({"operation":"announce", "owner":uuid(owner), "cast":hex(state.cast_id),
            "epoch":state.epoch, "accessRevision":settings.access_revision,
            "delaySeconds":settings.delay_seconds, "viewerCap":settings.viewer_cap, "passwordRequired":settings.password_required})).await?;
        Ok(())
    }
    pub async fn stop(&self, cast: [u8; 16]) -> Result<(), Error> {
        self.exchange(serde_json::json!({"operation":"stop", "cast":hex(cast)}))
            .await?;
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    purpose: String,
    node: String,
    account: String,
    cast: String,
    epoch: u64,
    #[serde(rename = "accessRevision")]
    access_revision: u64,
    reservation: String,
    #[serde(rename = "expiresAt")]
    expires_at: u64,
    chat: bool,
}
pub(super) fn decode_response(
    bytes: &[u8],
    node: [u8; 16],
    unix: u64,
    mono: u64,
) -> Result<VerifiedAdmission, Error> {
    if bytes.len() > 1024 {
        return Err(Error::Unavailable);
    }
    let value: Response = serde_json::from_slice(bytes).map_err(|_| Error::Unavailable)?;
    if value.purpose != "oteryn-cast-observer-v1"
        || parse_uuid(&value.node)? != node
        || value.epoch == 0
        || value.access_revision == 0
        || value.expires_at <= unix.saturating_add(1)
        || value.expires_at > unix.saturating_add(5)
    {
        return Err(Error::Unavailable);
    }
    let account = parse_uuid(&value.account)?;
    let reservation = parse_uuid(&value.reservation)?;
    let cast_id = parse_hex(&value.cast)?;
    if account == [0; 16] || reservation == [0; 16] || cast_id == [0; 16] {
        return Err(Error::Unavailable);
    }
    Ok(VerifiedAdmission {
        cast_id,
        epoch: value.epoch,
        account,
        reservation,
        access_revision: value.access_revision,
        chat: value.chat,
        expires_ms: mono
            .checked_add((value.expires_at - unix - 1) * 1000)
            .ok_or(Error::Invalid)?,
    })
}
pub(super) fn parse_uuid(s: &str) -> Result<[u8; 16], Error> {
    if s.len() != 36 || [8, 13, 18, 23].iter().any(|i| s.as_bytes()[*i] != b'-') {
        return Err(Error::Unavailable);
    }
    let hex: String = s.chars().filter(|c| *c != '-').collect();
    parse_hex(&hex)
}
fn parse_hex(s: &str) -> Result<[u8; 16], Error> {
    if s.len() != 32
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Unavailable);
    }
    let mut out = [0; 16];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).map_err(|_| Error::Unavailable)?;
    }
    Ok(out)
}
pub(super) fn hex(bytes: [u8; 16]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub(super) fn uuid(bytes: [u8; 16]) -> String {
    let s = hex(bytes);
    format!(
        "{}-{}-{}-{}-{}",
        &s[..8],
        &s[8..12],
        &s[12..16],
        &s[16..20],
        &s[20..]
    )
}
