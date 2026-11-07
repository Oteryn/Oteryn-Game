//! Party chat extends CHAT_V1 only when the session also selected PARTY_V1.
use super::*;

/// Optional Party room extension; requires CHAT_V1 and PARTY_V1.
pub const CAPABILITY_CHAT_PARTY_ROOM_V1: u32 = 240;

pub(super) fn encode_intent(text: &str) -> WireResult<Vec<u8>> {
    check_text(text, MAX_CHAT_TEXT_BYTES)?;
    let mut inner = Vec::new();
    push_message_field(&mut inner, 1, text.as_bytes());
    let mut out = Vec::new();
    push_message_field(&mut out, 14, &inner);
    Ok(out)
}

pub(super) fn decode_intent(input: &[u8]) -> WireResult<ChatIntent> {
    let mut text = None;
    read_fields(input, |field, wire, input, cursor| match (field, wire) {
        (1, 2) => set_once(&mut text, read_text(input, cursor, MAX_CHAT_TEXT_BYTES)?),
        _ => Err(ChatWireError::Malformed),
    })?;
    Ok(ChatIntent::Party {
        text: text.ok_or(ChatWireError::Malformed)?,
    })
}

pub(super) fn encode_line(name: &str, text: &str) -> WireResult<Vec<u8>> {
    check_text(name, MAX_CHAT_NAME_BYTES)?;
    check_text(text, MAX_CHAT_TEXT_BYTES)?;
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, 7);
    push_message_field(&mut out, 4, name.as_bytes());
    push_message_field(&mut out, 7, text.as_bytes());
    Ok(out)
}

pub(super) fn decode_line(input: &[u8]) -> WireResult<Option<ChatLine>> {
    let mut kind = None;
    read_fields(input, |field, wire, input, cursor| match (field, wire) {
        (1, 0) => set_once(&mut kind, read_uint32(input, cursor)?),
        (_, 0) => {
            read_varint(input, cursor)?;
            Ok(())
        }
        (_, 2) => {
            read_bytes(input, cursor)?;
            Ok(())
        }
        _ => Err(ChatWireError::Malformed),
    })?;
    if kind != Some(7) {
        return Ok(None);
    }
    let (mut name, mut text) = (None, None);
    read_fields(input, |field, wire, input, cursor| match (field, wire) {
        (1, 0) => {
            read_uint32(input, cursor)?;
            Ok(())
        }
        (4, 2) => set_once(&mut name, read_text(input, cursor, MAX_CHAT_NAME_BYTES)?),
        (7, 2) => set_once(&mut text, read_text(input, cursor, MAX_CHAT_TEXT_BYTES)?),
        _ => Err(ChatWireError::Malformed),
    })?;
    Ok(Some(ChatLine::Party {
        speaker_name: name.ok_or(ChatWireError::Malformed)?,
        text: text.ok_or(ChatWireError::Malformed)?,
    }))
}
