use super::{AttemptRef, GatewayLogin, PlatformClientError, RecoveryDeviceProof, ReleaseIdentity};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Serialize;
use sha2::{Digest, Sha256};

// Struct field order preserves the Platform golden request bytes. JCS is used only for the digest.
#[derive(Serialize)]
struct GatewayLoginRequest<'a> {
    protocol_version: u64,
    game_login_ticket: &'a str,
    attempt_ref: &'a str,
    character_id: &'a str,
    channel_id: Option<&'a str>,
    offer: GatewayOffer<'a>,
}

#[derive(Serialize)]
struct GatewayRecoveryRequest<'a> {
    protocol_version: u64,
    recovery_handle: &'a str,
    attempt_ref: &'a str,
    character_id: &'a str,
    offer: GatewayOffer<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_proof: Option<&'a RecoveryDeviceProof<'a>>,
}

#[derive(Serialize)]
pub(super) struct GatewayOffer<'a> {
    client_build: &'a str,
    client_platform: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    release: Option<ReleaseIdentity>,
    transports: [GatewayTransport; 1],
}

#[derive(Serialize)]
struct GatewayTransport {
    protocol_major: u64,
    transport_profile: u64,
    alpn: &'static str,
}

impl<'a> GatewayOffer<'a> {
    pub(super) fn new(
        client_build: &'a str,
        release: Option<ReleaseIdentity>,
    ) -> Result<Self, PlatformClientError> {
        if !super::is_visible_ascii(client_build, 64) {
            return Err(PlatformClientError::InvalidPayload);
        }
        Ok(Self {
            client_build,
            client_platform: "windows",
            release,
            transports: [GatewayTransport {
                protocol_major: 1,
                transport_profile: 1,
                alpn: super::GAME_ALPN,
            }],
        })
    }

    pub(super) fn digest(&self) -> Result<String, PlatformClientError> {
        let canonical = serde_jcs::to_vec(self).map_err(|_| PlatformClientError::InvalidPayload)?;
        Ok(Sha256::digest(canonical)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect())
    }

    pub(super) fn login(
        self,
        ticket: &'a str,
        login: &GatewayLogin<'a>,
        protocol_version: u64,
    ) -> Result<Vec<u8>, PlatformClientError> {
        if !matches!(protocol_version, 2 | 3)
            || super::parse_uuid_v7(login.character_id).is_none()
            || login
                .channel_id
                .is_some_and(|id| super::parse_uuid_v7(id).is_none())
        {
            return Err(PlatformClientError::InvalidPayload);
        }
        encode(&GatewayLoginRequest {
            protocol_version,
            game_login_ticket: ticket,
            attempt_ref: login.attempt_ref.as_str(),
            character_id: login.character_id,
            channel_id: login.channel_id,
            offer: self,
        })
    }

    pub(super) fn recover(
        self,
        handle: &'a str,
        attempt_ref: &'a AttemptRef,
        character_id: &'a str,
        device_proof: Option<&'a RecoveryDeviceProof<'a>>,
    ) -> Result<Vec<u8>, PlatformClientError> {
        if !handle
            .strip_prefix("otr1.")
            .is_some_and(|value| base64url(value, 43, 32))
            || super::parse_uuid_v7(character_id).is_none()
            || device_proof
                .is_some_and(|proof| !base64url(proof.kid, 43, 32) || !base64url(proof.sig, 86, 64))
        {
            return Err(PlatformClientError::InvalidPayload);
        }
        encode(&GatewayRecoveryRequest {
            protocol_version: 3,
            recovery_handle: handle,
            attempt_ref: attempt_ref.as_str(),
            character_id,
            offer: self,
            device_proof,
        })
    }
}

fn base64url(value: &str, encoded_len: usize, decoded_len: usize) -> bool {
    value.len() == encoded_len
        && URL_SAFE_NO_PAD
            .decode(value)
            .is_ok_and(|decoded| decoded.len() == decoded_len)
}

fn encode(request: &impl Serialize) -> Result<Vec<u8>, PlatformClientError> {
    let bytes = serde_json::to_vec(request).map_err(|_| PlatformClientError::InvalidPayload)?;
    if bytes.len() > super::MAX_LOGIN_REQUEST_BYTES {
        return Err(PlatformClientError::InvalidPayload);
    }
    Ok(bytes)
}
