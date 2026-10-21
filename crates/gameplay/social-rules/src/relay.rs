//! Pure World relay channel naming and refusal values.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

const CHANNEL_PREFIX: &str = "oteryn_chat_";

/// Why a relay line is refused. Every refusal drops the line; none reaches a client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayError {
    /// The line would not fit the sealed bound, or a field is out of bounds.
    Oversized,
    /// The World id cannot name a `NOTIFY` channel.
    BadChannel,
    /// No usable relay key: none configured, or the key set is invalid.
    NoKey,
    /// The `NOTIFY` text is not standard base64 or is too short to hold a sealed line.
    Malformed,
    /// The line names a `key_id` the node does not hold, or one retired over 24 hours ago.
    UnknownKey,
    /// Authentication failed: a forged, altered or foreign-channel line.
    Unauthentic,
    /// The plaintext is not a valid layout.
    BadPayload,
    /// `sent_at` is outside the freshness window.
    Stale,
    /// The line was sent before this process started listening. Nothing sent then was ever
    /// delivered to it, so only a replay of a captured line can arrive here.
    BeforeStart,
    /// The clock reading is past the lease stored for this listener: the line is dropped until
    /// the listener stores a new lease (`ReplayWindow::lease_due`).
    Unleased,
    /// The `message_id` was seen inside the window.
    Replayed,
    /// The de-duplication window is full of fresh lines.
    Saturated,
    /// The system random source failed.
    Random,
}

impl Display for RelayError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Oversized => "chat relay line is over the sealed bound",
            Self::BadChannel => "chat relay World id cannot name a channel",
            Self::NoKey => "chat relay has no usable key",
            Self::Malformed => "chat relay text is malformed",
            Self::UnknownKey => "chat relay line names an unknown or expired key",
            Self::Unauthentic => "chat relay line failed authentication",
            Self::BadPayload => "chat relay payload is malformed",
            Self::Stale => "chat relay line is outside the freshness window",
            Self::BeforeStart => "chat relay line predates the listener",
            Self::Unleased => "chat relay clock is past the stored lease",
            Self::Replayed => "chat relay line was already seen",
            Self::Saturated => "chat relay de-duplication window is full",
            Self::Random => "chat relay random source failed",
        })
    }
}

impl Error for RelayError {}

/// The `NOTIFY` channel of a World: `oteryn_chat_<32 hex digits>`, the canonical lower-case
/// UUIDv7 of the WorldId without its hyphens. The name is a safe unquoted identifier, 44 bytes,
/// and distinct per World. A WorldId is admitted exactly as `node/config.rs` (`v7_uuid`) admits
/// it: 36 characters with hyphens, version nibble `7` and RFC 9562 variant (`8`, `9`, `a`, `b`);
/// any other value is `BadChannel`, so no WorldId the node would refuse names a relay channel.
pub fn channel_name(world_id: &str) -> Result<String, RelayError> {
    let bytes = world_id.as_bytes();
    let canonical = bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => *byte == b'-',
            _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(byte),
        })
        && bytes[14] == b'7'
        && matches!(bytes[19], b'8' | b'9' | b'a' | b'b');
    if !canonical {
        return Err(RelayError::BadChannel);
    }
    let hex: String = world_id.chars().filter(|c| *c != '-').collect();
    Ok(format!("{CHANNEL_PREFIX}{hex}"))
}

#[cfg(test)]
mod tests;
