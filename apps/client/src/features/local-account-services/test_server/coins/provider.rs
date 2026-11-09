//! Test-only HMAC provider; its key is generated in memory, never loaded from repository config.
use super::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderError {
    Signature,
    Invalid,
    Binding,
    Replay,
}
impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ProviderError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EventKind {
    Paid,
    Refund,
    Chargeback,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Event {
    pub event_id: String,
    pub order_id: String,
    pub price: Price,
    pub kind: EventKind,
}

pub(crate) trait PaymentProvider {
    fn checkout_url(&self, order_id: &str) -> String;
    fn verify(&self, body: &[u8], signature: &str) -> Result<Event, ProviderError>;
}
pub(crate) struct SandboxProvider {
    key: [u8; 32],
    origin: String,
}
impl SandboxProvider {
    pub fn new(origin: String) -> TestResultWith<Self> {
        let mut key = [0; 32];
        oteryn_platform_client::native_login::secure_random(&mut key)?;
        Ok(Self { key, origin })
    }
    pub fn sign(&self, body: &[u8]) -> String {
        mac(&self.key, body)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}
impl PaymentProvider for SandboxProvider {
    fn checkout_url(&self, id: &str) -> String {
        format!("{}native-account/coin-checkout/{id}", self.origin)
    }
    fn verify(&self, body: &[u8], signature: &str) -> Result<Event, ProviderError> {
        if body.len() > 8192
            || signature.len() != 64
            || !signature
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(ProviderError::Signature);
        }
        let expected = mac(&self.key, body);
        let mut mismatch = 0_u8;
        for (index, pair) in signature.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            let text = std::str::from_utf8(pair).map_err(|_| ProviderError::Signature)?;
            let actual = u8::from_str_radix(text, 16).map_err(|_| ProviderError::Signature)?;
            mismatch |= expected[index] ^ actual;
        }
        if mismatch != 0 {
            return Err(ProviderError::Signature);
        }
        let event: Event = serde_json::from_slice(body).map_err(|_| ProviderError::Invalid)?;
        if !event.price.valid()
            || oteryn_platform_client::native_login::parse_uuid_v7(&event.order_id).is_none()
            || oteryn_platform_client::native_login::parse_uuid_v7(&event.event_id).is_none()
        {
            return Err(ProviderError::Invalid);
        }
        Ok(event)
    }
}
fn mac(key: &[u8], body: &[u8]) -> [u8; 32] {
    // Standard HMAC-SHA256 solely for this test adapter; RFC 4231 vector below guards the fixture.
    let normalized = if key.len() > 64 {
        Sha256::digest(key).to_vec()
    } else {
        key.to_vec()
    };
    let mut inner = [0x36; 64];
    let mut outer = [0x5c; 64];
    for (index, byte) in normalized.iter().enumerate() {
        inner[index] ^= byte;
        outer[index] ^= byte;
    }
    let mut first = Sha256::new();
    first.update(inner);
    first.update(body);
    let mut second = Sha256::new();
    second.update(outer);
    second.update(first.finalize());
    second.finalize().into()
}
#[test]
fn sandbox_mac_fixture_matches_public_rfc4231_vector() {
    let hex: String = mac(&[0x0b; 20], b"Hi There")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        hex,
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
}
