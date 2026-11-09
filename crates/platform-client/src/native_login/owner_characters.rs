//! Authenticated account characters and the player-selected Gateway login members.
use super::{
    AccessToken, AttemptRef, NativeLoginClient, NativeLoginError, parse_uuid_v7, read_bounded, run,
};
use crate::PlatformClientError;
use oteryn_foundation::CancellationToken;
use reqwest::StatusCode;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CharacterAvailability {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerCharacter {
    pub character_id: String,
    pub world_id: String,
    pub name: String,
    pub availability: CharacterAvailability,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnerCharacters {
    protocol_version: u64,
    characters: Vec<OwnerCharacter>,
}

/// The Gateway login request's caller-chosen members (N4-P §3.1).
#[derive(Debug, Clone)]
pub struct GatewayLogin<'a> {
    pub attempt_ref: &'a AttemptRef,
    /// Canonical lowercase UUIDv7 chosen from the owner character feed.
    pub character_id: &'a str,
    /// Optional canonical UUIDv7 channel selected from the directory.
    pub channel_id: Option<&'a str>,
    /// 1..64 visible ASCII; diagnostics only.
    pub client_build: &'a str,
}

impl NativeLoginClient {
    /// Reads the authenticated owner's characters without consuming the OAuth token.
    pub async fn owner_characters(
        &self,
        token: &AccessToken,
        cancellation: CancellationToken,
    ) -> Result<Vec<OwnerCharacter>, NativeLoginError> {
        let endpoint = self
            .platform
            .join("/api/v1/game-auth/native-characters")
            .map_err(|_| PlatformClientError::InvalidBaseUrl)?;
        run(cancellation, async {
            let response = self
                .client
                .get(endpoint)
                .bearer_auth(token.0.expose())
                .send()
                .await
                .map_err(|_| PlatformClientError::Request)?;
            match response.status() {
                StatusCode::OK => {}
                StatusCode::UNAUTHORIZED => return Err(NativeLoginError::TokenRefused),
                status => return Err(PlatformClientError::HttpStatus(status.as_u16()).into()),
            }
            let body = read_bounded(response, 16_384).await?;
            let feed: OwnerCharacters =
                serde_json::from_slice(&body).map_err(|_| PlatformClientError::InvalidPayload)?;
            if feed.protocol_version != 2
                || feed.characters.iter().any(|character| {
                    parse_uuid_v7(&character.character_id).is_none()
                        || parse_uuid_v7(&character.world_id).is_none()
                        || character.name.is_empty()
                })
            {
                return Err(PlatformClientError::InvalidPayload.into());
            }
            Ok(feed.characters)
        })
        .await
    }
}

mod device_transport;
