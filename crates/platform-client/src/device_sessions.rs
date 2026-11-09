//! Accepted native remembered-device protocol 1. Secrets never implement revealing Debug.
use crate::PlatformClientError;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use std::fmt;
#[derive(Clone, PartialEq, Eq)]
pub struct DeviceCredential(String);
impl DeviceCredential {
    pub fn parse(value: &str) -> Result<Self, PlatformClientError> {
        let encoded = value
            .strip_prefix("otd1.")
            .ok_or(PlatformClientError::InvalidPayload)?;
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| PlatformClientError::InvalidPayload)?;
        if bytes.len() != 32 || value.len() != 48 || URL_SAFE_NO_PAD.encode(bytes) != encoded {
            return Err(PlatformClientError::InvalidPayload);
        }
        Ok(Self(value.into()))
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for DeviceCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DeviceCredential([REDACTED])")
    }
}
impl Serialize for DeviceCredential {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for DeviceCredential {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceSession {
    pub protocol_version: u32,
    pub device_credential: DeviceCredential,
    pub family_id: String,
    pub absolute_expires_at: u64,
    pub idle_expires_at: u64,
}
impl DeviceSession {
    pub fn validate(&self, now: u64) -> Result<(), PlatformClientError> {
        if self.protocol_version != 1
            || !canonical_uuid(&self.family_id)
            || self.idle_expires_at <= now
            || self.idle_expires_at.saturating_sub(now) > 7 * 86400
            || self.absolute_expires_at < self.idle_expires_at
            || self.absolute_expires_at.saturating_sub(now) > 30 * 86400
        {
            return Err(PlatformClientError::InvalidPayload);
        }
        Ok(())
    }
    pub fn successor(&self, next: &Self, now: u64) -> Result<(), PlatformClientError> {
        next.validate(now)?;
        if self.family_id != next.family_id
            || self.absolute_expires_at != next.absolute_expires_at
            || self.device_credential == next.device_credential
        {
            return Err(PlatformClientError::InvalidPayload);
        }
        Ok(())
    }
    pub fn vault_bytes(&self) -> Result<Vec<u8>, PlatformClientError> {
        serde_json::to_vec(self).map_err(|_| PlatformClientError::InvalidPayload)
    }
    pub fn from_vault(bytes: &[u8], now: u64) -> Result<Self, PlatformClientError> {
        if bytes.len() > 1024 {
            return Err(PlatformClientError::InvalidPayload);
        }
        let value: Self =
            serde_json::from_slice(bytes).map_err(|_| PlatformClientError::InvalidPayload)?;
        value.validate(now)?;
        Ok(value)
    }
}
pub fn now() -> Result<u64, PlatformClientError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|t| t.as_secs())
        .map_err(|_| PlatformClientError::InvalidPayload)
}
/// A session-class key never leaves process memory. Remembered enrollment uses the
/// accepted original unbound variant, without inventing a non-exportable key store.
pub struct SessionKey(ed25519_dalek::SigningKey);
impl SessionKey {
    pub fn generate() -> Result<Self, PlatformClientError> {
        let mut seed = [0_u8; 32];
        crate::native_login::secure_random(&mut seed)?;
        Ok(Self(ed25519_dalek::SigningKey::from_bytes(&seed)))
    }
    pub fn enrollment(
        &self,
        token: &str,
        client_id: &str,
    ) -> Result<serde_json::Value, PlatformClientError> {
        use ed25519_dalek::Signer;
        use sha2::{Digest, Sha256};
        let public = URL_SAFE_NO_PAD.encode(self.0.verifying_key().as_bytes());
        let digest = Sha256::digest(token.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        // serde_json's map is sorted by key: precisely the contract's JCS lexical order.
        let input = serde_json::json!({"access_token_sha256":digest,"client_id":client_id,"persistence":"session","public_key":public,"purpose":"oteryn-native-device-enroll-v1"});
        let proof = self
            .0
            .sign(&serde_json::to_vec(&input).map_err(|_| PlatformClientError::InvalidPayload)?);
        Ok(
            serde_json::json!({"protocol_version":1,"remember_device":false,"device_public_key":public,"enrollment_proof":URL_SAFE_NO_PAD.encode(proof.to_bytes())}),
        )
    }
}

/// Vault namespace binds the exact canonical trusted origin and first-party client UUID.
pub fn vault_key(origin: &str, client: &str) -> Result<String, PlatformClientError> {
    use sha2::{Digest, Sha256};
    let url = url::Url::parse(origin).map_err(|_| PlatformClientError::InvalidBaseUrl)?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || !canonical_uuid(client)
    {
        return Err(PlatformClientError::InvalidBaseUrl);
    }
    let input = format!("{}\n{client}", url.origin().ascii_serialization());
    Ok(Sha256::digest(input.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

pub fn canonical_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}
