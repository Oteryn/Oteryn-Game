//! Bounded asynchronous Platform directory client.

use oteryn_foundation::{BoundedText, Cancellable, CancellationToken, cancellable};
use oteryn_platform_contracts::{
    ClientDirectoryEpoch, DirectoryChannel, DirectoryChannelRef, DirectoryCharacter,
    DirectoryCharacterRef, DirectorySnapshot, DirectoryWorld, DirectoryWorldRef,
};
use reqwest::{Client, Url, redirect::Policy};
use serde_json::{Map, Value};
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 1_048_576;
pub const MAX_WORLDS: usize = 256;
pub const MAX_CHANNELS_PER_WORLD: usize = 128;
pub const MAX_CHARACTERS_PER_WORLD: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformClientError {
    InvalidBaseUrl,
    InsecureBaseUrl,
    ClientCreation,
    Cancelled,
    Request,
    HttpStatus(u16),
    ResponseTooLarge,
    InvalidPayload,
    ForbiddenGameplayField,
}

impl Display for PlatformClientError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBaseUrl => formatter.write_str("invalid Platform base URL"),
            Self::InsecureBaseUrl => formatter.write_str("Platform URL must use HTTPS"),
            Self::ClientCreation => formatter.write_str("Platform HTTP client creation failed"),
            Self::Cancelled => formatter.write_str("Platform operation was cancelled"),
            Self::Request => formatter.write_str("Platform request failed"),
            Self::HttpStatus(status) => write!(formatter, "Platform returned HTTP status {status}"),
            Self::ResponseTooLarge => formatter.write_str("Platform response exceeded its bound"),
            Self::InvalidPayload => formatter.write_str("Platform response payload is invalid"),
            Self::ForbiddenGameplayField => {
                formatter.write_str("Platform directory exposed a forbidden gameplay field")
            }
        }
    }
}

impl std::error::Error for PlatformClientError {}

#[derive(Debug, Clone)]
pub struct PlatformClientConfig {
    base_url: Url,
    max_response_bytes: usize,
    connect_timeout: Duration,
    request_timeout: Duration,
}

impl PlatformClientConfig {
    pub fn new(base_url: &str) -> Result<Self, PlatformClientError> {
        let base_url =
            Url::parse(base_url).map_err(|_error| PlatformClientError::InvalidBaseUrl)?;
        let loopback = matches!(base_url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
        if base_url.scheme() != "https" && !(loopback && base_url.scheme() == "http") {
            return Err(PlatformClientError::InsecureBaseUrl);
        }
        Ok(Self {
            base_url,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(15),
        })
    }

    #[must_use]
    pub fn with_max_response_bytes(mut self, max_response_bytes: usize) -> Self {
        self.max_response_bytes = max_response_bytes.clamp(1, DEFAULT_MAX_RESPONSE_BYTES);
        self
    }
}

pub struct PlatformClient {
    client: Client,
    config: PlatformClientConfig,
}

impl PlatformClient {
    pub fn new(config: PlatformClientConfig) -> Result<Self, PlatformClientError> {
        let client = Client::builder()
            .redirect(Policy::none())
            .no_proxy()
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .build()
            .map_err(|_error| PlatformClientError::ClientCreation)?;
        Ok(Self { client, config })
    }

    pub async fn fetch_directory(
        &self,
        cancellation: CancellationToken,
    ) -> Result<DirectorySnapshot, PlatformClientError> {
        let endpoint = self
            .config
            .base_url
            .join("v1/client/directory")
            .map_err(|_error| PlatformClientError::InvalidBaseUrl)?;
        let request = async {
            let mut response = self
                .client
                .get(endpoint)
                .send()
                .await
                .map_err(|_error| PlatformClientError::Request)?;
            if !response.status().is_success() {
                return Err(PlatformClientError::HttpStatus(response.status().as_u16()));
            }
            if response
                .content_length()
                .is_some_and(|length| length > self.config.max_response_bytes as u64)
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
                if next_len > self.config.max_response_bytes {
                    return Err(PlatformClientError::ResponseTooLarge);
                }
                body.extend_from_slice(&chunk);
            }
            parse_directory(&body)
        };
        match cancellable(cancellation, request).await {
            Cancellable::Completed(result) => result,
            Cancellable::Cancelled => Err(PlatformClientError::Cancelled),
        }
    }
}

pub fn parse_directory(bytes: &[u8]) -> Result<DirectorySnapshot, PlatformClientError> {
    if bytes.len() > DEFAULT_MAX_RESPONSE_BYTES {
        return Err(PlatformClientError::ResponseTooLarge);
    }
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_error| PlatformClientError::InvalidPayload)?;
    if contains_forbidden_gameplay_field(&value) {
        return Err(PlatformClientError::ForbiddenGameplayField);
    }
    let root = value
        .as_object()
        .ok_or(PlatformClientError::InvalidPayload)?;
    let epoch = root
        .get("epoch")
        .and_then(Value::as_u64)
        .ok_or(PlatformClientError::InvalidPayload)?;
    let world_values = root
        .get("worlds")
        .and_then(Value::as_array)
        .ok_or(PlatformClientError::InvalidPayload)?;
    if world_values.len() > MAX_WORLDS {
        return Err(PlatformClientError::InvalidPayload);
    }
    let mut worlds = Vec::with_capacity(world_values.len());
    for world_value in world_values {
        worlds.push(parse_world(
            world_value
                .as_object()
                .ok_or(PlatformClientError::InvalidPayload)?,
        )?);
    }
    Ok(DirectorySnapshot {
        epoch: ClientDirectoryEpoch::new(epoch),
        worlds,
    })
}

fn parse_world(object: &Map<String, Value>) -> Result<DirectoryWorld, PlatformClientError> {
    let channel_values = object
        .get("channels")
        .and_then(Value::as_array)
        .ok_or(PlatformClientError::InvalidPayload)?;
    let character_values = object
        .get("characters")
        .and_then(Value::as_array)
        .ok_or(PlatformClientError::InvalidPayload)?;
    if channel_values.len() > MAX_CHANNELS_PER_WORLD
        || character_values.len() > MAX_CHARACTERS_PER_WORLD
    {
        return Err(PlatformClientError::InvalidPayload);
    }
    let mut channels = Vec::with_capacity(channel_values.len());
    for value in channel_values {
        let entry = value
            .as_object()
            .ok_or(PlatformClientError::InvalidPayload)?;
        channels.push(DirectoryChannel {
            channel_ref: DirectoryChannelRef::new(required_string(entry, "id")?)
                .map_err(|_error| PlatformClientError::InvalidPayload)?,
            display_name: BoundedText::new(required_string(entry, "name")?, 96)
                .map_err(|_error| PlatformClientError::InvalidPayload)?,
        });
    }
    let mut characters = Vec::with_capacity(character_values.len());
    for value in character_values {
        let entry = value
            .as_object()
            .ok_or(PlatformClientError::InvalidPayload)?;
        characters.push(DirectoryCharacter {
            character_ref: DirectoryCharacterRef::new(required_string(entry, "id")?)
                .map_err(|_error| PlatformClientError::InvalidPayload)?,
            display_name: BoundedText::new(required_string(entry, "name")?, 96)
                .map_err(|_error| PlatformClientError::InvalidPayload)?,
        });
    }
    Ok(DirectoryWorld {
        world_ref: DirectoryWorldRef::new(required_string(object, "id")?)
            .map_err(|_error| PlatformClientError::InvalidPayload)?,
        display_name: BoundedText::new(required_string(object, "name")?, 96)
            .map_err(|_error| PlatformClientError::InvalidPayload)?,
        channels,
        characters,
    })
}

fn required_string<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a str, PlatformClientError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or(PlatformClientError::InvalidPayload)
}

fn contains_forbidden_gameplay_field(value: &Value) -> bool {
    const FORBIDDEN: [&str; 12] = [
        "host",
        "port",
        "endpoint",
        "endpoint_uri",
        "protocol",
        "protocol_profile",
        "ticket",
        "credential",
        "game_session",
        "admission",
        "route",
        "address",
    ];
    match value {
        Value::Object(object) => object.iter().any(|(key, child)| {
            FORBIDDEN.contains(&key.as_str()) || contains_forbidden_gameplay_field(child)
        }),
        Value::Array(values) => values.iter().any(contains_forbidden_gameplay_field),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAFE: &[u8] = br#"{"epoch":7,"worlds":[{"id":"w-alpha","name":"Alpha","channels":[{"id":"c-1","name":"Default"}],"characters":[{"id":"char-1","name":"Ari"}]}]}"#;

    #[test]
    fn safe_directory_contains_no_route() -> Result<(), PlatformClientError> {
        let directory = parse_directory(SAFE)?;
        assert_eq!(directory.epoch.get(), 7);
        assert_eq!(directory.worlds.len(), 1);
        Ok(())
    }

    #[test]
    fn gameplay_fields_fail_closed() {
        let payload = br#"{"epoch":1,"worlds":[],"host":"127.0.0.1"}"#;
        assert_eq!(
            parse_directory(payload),
            Err(PlatformClientError::ForbiddenGameplayField)
        );
    }

    #[test]
    fn non_loopback_http_is_rejected() {
        assert!(matches!(
            PlatformClientConfig::new("http://example.invalid/"),
            Err(PlatformClientError::InsecureBaseUrl)
        ));
    }
}
