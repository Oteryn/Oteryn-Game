//! TUTORIAL-1 tutorial notice payloads (owner Q3 decision on #2186, 2026-10-19).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/tutorial_notices_v1.proto`. Capability 31
//! `TUTORIAL_NOTICES_V1` and state domain 23 `TUTORIAL_NOTICES` are registered in
//! `PROTOCOL_OTERYN_V1_REGISTRY.json`. The capability has no command type and no requirement.
//! The domain carries no durable state: the snapshot only says the notice stream is live, and one
//! delta follows each committed Newhaven tutorial transition that shows the player something,
//! either a tutorial hint (a small number the client maps to its own hint window) or one line of
//! status text. The notice is presentation only; the tutorial progress itself is server state.
//!
//! Decoding is strict, and encoding refuses the same values as a server fault before any byte is
//! emitted. The following all fail closed: a hint of zero, a hint above 255, an empty or
//! oversized status text, invalid UTF-8, a control character in the text, both or neither of the
//! two fields, a payload over its bound, and unknown or repeated fields.

pub use crate::charm_wire::CyclopediaWireError as TutorialNoticesWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_varint_field, read_bytes, read_varint, set_once,
};

/// Registered capability `TUTORIAL_NOTICES_V1`: no command type, state domain 23.
pub const CAPABILITY_TUTORIAL_NOTICES_V1: u32 = 31;
/// The capability requires no other capability: a notice names no World position or entity.
pub const CAPABILITY_TUTORIAL_NOTICES_V1_REQUIRES: [u32; 0] = [];
/// Registered state domain `TUTORIAL_NOTICES` (capability 31).
pub const STATE_DOMAIN_TUTORIAL_NOTICES: u32 = 23;
/// Delta type 1 `TUTORIAL_NOTICE_DELTA_V1`: one hint or one status line.
pub const DELTA_TYPE_TUTORIAL_NOTICE_V1: u32 = 1;
/// Snapshot type 1 `TUTORIAL_NOTICES_SNAPSHOT_V1`: the stream is live; no notice is replayed.
pub const SNAPSHOT_TYPE_TUTORIAL_NOTICES_V1: u32 = 1;

/// The longest status text in UTF-8 bytes.
pub const MAX_TUTORIAL_STATUS_TEXT_BYTES: usize = 255;
/// Worst case 258: tag 1 + length 2 + 255 text bytes.
pub const MAX_TUTORIAL_NOTICE_BYTES: usize = 260;
/// The snapshot is always the 2 bytes `[0x08, 0x01]`.
pub const MAX_TUTORIAL_NOTICES_SNAPSHOT_BYTES: usize = 8;

const _: () = assert!(MAX_TUTORIAL_NOTICE_BYTES <= crate::MAX_STATE_DELTA_PAYLOAD_BYTES);
const _: () = assert!(MAX_TUTORIAL_STATUS_TEXT_BYTES + 3 <= MAX_TUTORIAL_NOTICE_BYTES);

/// The snapshot status: 1 `LIVE`, the only value.
const SNAPSHOT_STATUS_LIVE: u64 = 1;

/// `TutorialNoticeV1`: exactly one of the two notices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TutorialNotice {
    /// A tutorial hint number, 1..=255. The client maps it to its own hint window.
    Hint(u8),
    /// One line of status text, 1..=255 UTF-8 bytes without control characters.
    StatusText(String),
}

fn valid_text(text: &str) -> WireResult<()> {
    if text.len() > MAX_TUTORIAL_STATUS_TEXT_BYTES {
        return Err(TutorialNoticesWireError::LimitExceeded);
    }
    if text.is_empty() || text.chars().any(char::is_control) {
        return Err(TutorialNoticesWireError::Malformed);
    }
    Ok(())
}

/// Encodes the snapshot (type 1) payload.
#[must_use]
pub fn encode_tutorial_notices_snapshot() -> Vec<u8> {
    let mut output = Vec::with_capacity(2);
    push_varint_field(&mut output, 1, SNAPSHOT_STATUS_LIVE);
    output
}

pub fn decode_tutorial_notices_snapshot(payload: &[u8]) -> WireResult<()> {
    if payload.len() > MAX_TUTORIAL_NOTICES_SNAPSHOT_BYTES {
        return Err(TutorialNoticesWireError::LimitExceeded);
    }
    let mut status = None;
    let mut cursor = 0;
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x08 => set_once(&mut status, read_varint(payload, &mut cursor)?)?,
            _ => return Err(TutorialNoticesWireError::Malformed),
        }
    }
    match status {
        Some(SNAPSHOT_STATUS_LIVE) => Ok(()),
        _ => Err(TutorialNoticesWireError::Malformed),
    }
}

/// Encodes the notice delta (type 1). A notice outside the bounds is a server fault.
pub fn encode_tutorial_notice(notice: &TutorialNotice) -> WireResult<Vec<u8>> {
    let mut output = Vec::with_capacity(MAX_TUTORIAL_NOTICE_BYTES);
    match notice {
        TutorialNotice::Hint(0) => return Err(TutorialNoticesWireError::Malformed),
        TutorialNotice::Hint(hint) => push_varint_field(&mut output, 1, u64::from(*hint)),
        TutorialNotice::StatusText(text) => {
            valid_text(text)?;
            push_message_field(&mut output, 2, text.as_bytes());
        }
    }
    Ok(output)
}

pub fn decode_tutorial_notice(payload: &[u8]) -> WireResult<TutorialNotice> {
    if payload.len() > MAX_TUTORIAL_NOTICE_BYTES {
        return Err(TutorialNoticesWireError::LimitExceeded);
    }
    let (mut hint, mut text) = (None, None);
    let mut cursor = 0;
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x08 => set_once(&mut hint, read_varint(payload, &mut cursor)?)?,
            0x12 => {
                let bytes = read_bytes(payload, &mut cursor)?;
                let value =
                    std::str::from_utf8(bytes).map_err(|_| TutorialNoticesWireError::Malformed)?;
                set_once(&mut text, value.to_owned())?;
            }
            _ => return Err(TutorialNoticesWireError::Malformed),
        }
    }
    match (hint, text) {
        (Some(hint), None) => match u8::try_from(hint) {
            Ok(hint) if hint != 0 => Ok(TutorialNotice::Hint(hint)),
            _ => Err(TutorialNoticesWireError::Malformed),
        },
        (None, Some(text)) => {
            valid_text(&text)?;
            Ok(TutorialNotice::StatusText(text))
        }
        _ => Err(TutorialNoticesWireError::Malformed),
    }
}

#[cfg(test)]
#[path = "tutorial_notices_tests.rs"]
mod tests;
