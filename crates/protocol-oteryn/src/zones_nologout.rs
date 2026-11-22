//! ZONES-NOLOGOUT-1 and RAIDS-GLOBAL: the existing server-message domain.
//! Legacy status-text bytes are unchanged; raid warnings and advances retain
//! their source kind without changing the separate tutorial notice codec.
pub use crate::charm_wire::CyclopediaWireError as ServerMessageError;
use crate::charm_wire::{WireResult, push_message_field, read_bytes, read_varint};

pub const CAPABILITY_ZONES_NOLOGOUT_V1: u32 = 92;
pub const STATE_DOMAIN_SERVER_MESSAGES: u32 = 57;
pub const MAX_SERVER_MOTD_BYTES: usize = 255;
pub const SERVER_MESSAGES_EMPTY_SNAPSHOT: &[u8] = &[8, 1];
pub const NO_LOGOUT_MESSAGE: &str = "You can not logout here.";
pub const IN_FIGHT_MESSAGE: &str = "You may not logout during or immediately after a fight!";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerMessage {
    StatusText(String),
    Warning(String),
    Advance(String),
}

pub fn encode_server_message(message: &ServerMessage) -> WireResult<Vec<u8>> {
    let (field, text) = match message {
        ServerMessage::StatusText(text) => (2, text),
        ServerMessage::Warning(text) => (3, text),
        ServerMessage::Advance(text) => (4, text),
    };
    if text.is_empty() || text.len() > MAX_SERVER_MOTD_BYTES {
        return Err(ServerMessageError::Malformed);
    }
    let mut bytes = Vec::new();
    push_message_field(&mut bytes, field, text.as_bytes());
    Ok(bytes)
}

pub fn decode_server_message(bytes: &[u8]) -> WireResult<ServerMessage> {
    if bytes.len() > 260 {
        return Err(ServerMessageError::LimitExceeded);
    }
    let mut cursor = 0;
    let tag = read_varint(bytes, &mut cursor)?;
    if !matches!(tag, 0x12 | 0x1a | 0x22) {
        return Err(ServerMessageError::Malformed);
    }
    let text = read_bytes(bytes, &mut cursor)?;
    if text.is_empty() || text.len() > MAX_SERVER_MOTD_BYTES || cursor != bytes.len() {
        return Err(ServerMessageError::Malformed);
    }
    let text = std::str::from_utf8(text)
        .map_err(|_| ServerMessageError::Malformed)?
        .to_owned();
    Ok(match tag {
        0x12 => ServerMessage::StatusText(text),
        0x1a => ServerMessage::Warning(text),
        0x22 => ServerMessage::Advance(text),
        _ => return Err(ServerMessageError::Malformed),
    })
}

#[cfg(test)]
#[path = "zones_nologout_tests.rs"]
mod tests;

pub const CAPABILITY_PLAYER_STATUS_V1: u32 = 600;
pub const COMMAND_TYPE_PLAYER_STATUS_QUERY: u32 = 6363;

pub fn decode_player_status_query(
    bytes: &[u8],
) -> Option<crate::world_spatial_entities::EntityRef> {
    let (&identity, generation) = bytes.split_first_chunk::<16>()?;
    let generation = u64::from_be_bytes(generation.try_into().ok()?);
    (identity != [0; 16] && generation != 0).then_some(crate::world_spatial_entities::EntityRef {
        identity,
        generation,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PlayerPresence {
    Online = 1,
    Away = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerStatus {
    pub presence: PlayerPresence,
    pub no_logout: bool,
}

impl PlayerStatus {
    pub fn encode(self) -> [u8; 2] {
        [self.presence as u8, u8::from(self.no_logout)]
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let [presence, no_logout @ 0..=1] = *bytes else {
            return None;
        };
        Some(Self {
            presence: match presence {
                1 => PlayerPresence::Online,
                2 => PlayerPresence::Away,
                _ => return None,
            },
            no_logout: no_logout == 1,
        })
    }
}
