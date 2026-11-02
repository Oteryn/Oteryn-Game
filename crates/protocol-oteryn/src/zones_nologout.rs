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
