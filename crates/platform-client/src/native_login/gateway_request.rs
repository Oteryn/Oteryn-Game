use super::{GatewayLogin, PlatformClientError};
use serde::Serialize;

// Struct field order preserves the Platform golden request bytes. Serializing
// through serde_json::Value would sort members and lose that order.
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
struct GatewayOffer<'a> {
    client_build: &'a str,
    client_platform: &'a str,
    transports: [GatewayTransport<'a>; 1],
}

#[derive(Serialize)]
struct GatewayTransport<'a> {
    protocol_major: u64,
    transport_profile: u64,
    alpn: &'a str,
}

pub(super) fn body(
    ticket_secret: &str,
    login: &GatewayLogin<'_>,
    protocol_version: u64,
    game_alpn: &str,
) -> Result<Vec<u8>, PlatformClientError> {
    serde_json::to_vec(&GatewayLoginRequest {
        protocol_version,
        game_login_ticket: ticket_secret,
        attempt_ref: login.attempt_ref.as_str(),
        character_id: login.character_id,
        channel_id: login.channel_id,
        offer: GatewayOffer {
            client_build: login.client_build,
            client_platform: "windows",
            transports: [GatewayTransport {
                protocol_major: 1,
                transport_profile: 1,
                alpn: game_alpn,
            }],
        },
    })
    .map_err(|_error| PlatformClientError::InvalidPayload)
}
