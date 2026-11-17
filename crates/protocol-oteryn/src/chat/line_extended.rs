//! WU26 CHAT_V1 line extensions: the private echo (capability 493), the private-channel notice
//! (capability 495) and the line sequence (capability 496).
//!
//! - Capability 493 `CHAT_PRIVATE_ECHO_V1` requires 7. The sender of an accepted private message
//!   receives it back as line kind 2 with field 12 `recipient_name` (1..=120 bytes, encoded like
//!   field 4). Field 12 belongs to kind 2 only and is sent only on that echo.
//! - Capability 495 `CHAT_CHANNEL_PARTICIPANTS_V1` requires 80. Line kind 8 `ChannelNotice`
//!   carries field 4 (the member's name), field 11 (channel 1..=65535, as on kind 6) and field 14
//!   (1 joined, 2 left, 3 excluded) only. Kind 7 is the party line (capability 240), so the
//!   notice takes the next kind. The channel list half is in `private_channel_extended`.
//! - Capability 496 `CHAT_LINE_SEQUENCE_V1` requires 7. Every domain 12 line delta (type 1) of the
//!   session carries field 13 `line_sequence`: the delta's own new domain 12 revision. It is
//!   never 0 and strictly increasing per GameSession, across resume and transfer, with gaps where
//!   other domain 12 deltas (rooms, channel list, typing) take a revision.
//!
//! Without selection every line byte is unchanged, and the legacy [`decode_chat_line`] still
//! refuses fields 12..=14 and kind 8. [`decode_chat_line_extended`] accepts every legacy line too.
use super::*;

/// Registered capability `CHAT_PRIVATE_ECHO_V1` (requires 7).
pub const CAPABILITY_CHAT_PRIVATE_ECHO_V1: u32 = 493;
/// Registered capability `CHAT_LINE_SEQUENCE_V1` (requires 7).
pub const CAPABILITY_CHAT_LINE_SEQUENCE_V1: u32 = 496;
/// A private echo at its largest: kind 2, speaker name 122, text 1,023, recipient name 122 and
/// line sequence 1 + 10 bytes. Every other extended line is shorter.
pub const MAX_CHAT_LINE_EXTENDED_BYTES: usize = 1_280;

const FIELD_RECIPIENT_NAME: u64 = 12;
const FIELD_LINE_SEQUENCE: u64 = 13;
const FIELD_NOTICE_KIND: u64 = 14;
const LINE_KIND_PRIVATE: u64 = 2;
const LINE_KIND_CHANNEL_NOTICE: u32 = 8;

/// Capability 493: the sender's own private message, as the relay carried it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateEcho {
    /// The sender's own name (field 4).
    pub speaker_name: String,
    /// The recipient the sender named (field 12).
    pub recipient_name: String,
    pub text: String,
}

/// Field 14 of line kind 8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelNoticeKind {
    Joined = 1,
    Left = 2,
    Excluded = 3,
}

impl ChannelNoticeKind {
    fn from_wire(value: u32) -> WireResult<Self> {
        match value {
            1 => Ok(Self::Joined),
            2 => Ok(Self::Left),
            3 => Ok(Self::Excluded),
            _ => Err(ChatWireError::Malformed),
        }
    }
}

/// Capability 495, line kind 8: a member joined, left or was excluded from a private channel
/// the session is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelNotice {
    pub channel_id: u16,
    pub kind: ChannelNoticeKind,
    pub name: String,
}

/// What one extended line carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatLineBody {
    /// Any legacy CHAT_V1 line.
    Line(ChatLine),
    PrivateEcho(PrivateEcho),
    ChannelNotice(ChannelNotice),
}

/// One domain 12 line delta as a session with the WU26 capabilities decodes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedChatLine {
    pub body: ChatLineBody,
    /// Capability 496: field 13.
    pub line_sequence: Option<u64>,
}

/// Encodes a private echo: the kind 2 line of the same speaker and text, then field 12.
pub fn encode_private_echo(echo: &PrivateEcho) -> WireResult<Vec<u8>> {
    check_text(&echo.speaker_name, MAX_CHAT_NAME_BYTES)?;
    check_text(&echo.text, MAX_CHAT_TEXT_BYTES)?;
    check_text(&echo.recipient_name, MAX_CHAT_NAME_BYTES)?;
    let mut out = Vec::with_capacity(MAX_CHAT_LINE_EXTENDED_BYTES);
    push_varint_field(&mut out, 1, LINE_KIND_PRIVATE);
    push_message_field(&mut out, 4, echo.speaker_name.as_bytes());
    push_message_field(&mut out, 7, echo.text.as_bytes());
    push_message_field(
        &mut out,
        FIELD_RECIPIENT_NAME,
        echo.recipient_name.as_bytes(),
    );
    Ok(out)
}

/// Encodes a channel notice (line kind 8).
pub fn encode_channel_notice(notice: &ChannelNotice) -> WireResult<Vec<u8>> {
    if notice.channel_id == 0 {
        return Err(ChatWireError::Malformed);
    }
    check_text(&notice.name, MAX_CHAT_NAME_BYTES)?;
    let mut out = Vec::with_capacity(MAX_CHAT_NAME_BYTES + 16);
    push_varint_field(&mut out, 1, u64::from(LINE_KIND_CHANNEL_NOTICE));
    push_message_field(&mut out, 4, notice.name.as_bytes());
    push_varint_field(&mut out, 11, u64::from(notice.channel_id));
    push_varint_field(&mut out, FIELD_NOTICE_KIND, notice.kind as u64);
    Ok(out)
}

/// Appends field 13 to an encoded line (capability 496). A zero sequence is a server fault.
pub fn append_line_sequence(line: &mut Vec<u8>, line_sequence: u64) -> WireResult<()> {
    if line_sequence == 0 {
        return Err(ChatWireError::Malformed);
    }
    push_varint_field(line, FIELD_LINE_SEQUENCE, line_sequence);
    Ok(())
}

/// Decodes any line delta of a session with capabilities 493, 495 or 496: fields 12..=14 are
/// taken out, and the rest must be a legacy line or a kind 8 notice.
pub fn decode_chat_line_extended(payload: &[u8]) -> WireResult<ExtendedChatLine> {
    if payload.len() > MAX_CHAT_LINE_EXTENDED_BYTES {
        return Err(ChatWireError::LimitExceeded);
    }
    let (mut kind, mut recipient, mut sequence, mut notice_kind) = (None, None, None, None);
    let mut rest = Vec::with_capacity(payload.len());
    let mut cursor = 0;
    while cursor < payload.len() {
        let start = cursor;
        let key = read_varint(payload, &mut cursor)?;
        match (key >> 3, key & 0x07) {
            (FIELD_RECIPIENT_NAME, 2) => set_once(
                &mut recipient,
                read_text(payload, &mut cursor, MAX_CHAT_NAME_BYTES)?,
            )?,
            (FIELD_LINE_SEQUENCE, 0) => {
                set_once(&mut sequence, read_varint(payload, &mut cursor)?)?
            }
            (FIELD_NOTICE_KIND, 0) => {
                set_once(&mut notice_kind, read_uint32(payload, &mut cursor)?)?;
            }
            (field, wire) => {
                match wire {
                    0 if field == 1 => set_once(&mut kind, read_uint32(payload, &mut cursor)?)?,
                    0 => {
                        read_varint(payload, &mut cursor)?;
                    }
                    2 => {
                        read_bytes(payload, &mut cursor)?;
                    }
                    _ => return Err(ChatWireError::Malformed),
                }
                rest.extend_from_slice(payload.get(start..cursor).ok_or(ChatWireError::Malformed)?);
            }
        }
    }
    if sequence == Some(0) {
        return Err(ChatWireError::Malformed);
    }
    let body = if kind == Some(LINE_KIND_CHANNEL_NOTICE) {
        if recipient.is_some() {
            return Err(ChatWireError::Malformed);
        }
        let kind = ChannelNoticeKind::from_wire(notice_kind.ok_or(ChatWireError::Malformed)?)?;
        ChatLineBody::ChannelNotice(decode_notice(&rest, kind)?)
    } else {
        if notice_kind.is_some() {
            return Err(ChatWireError::Malformed);
        }
        match (decode_chat_line(&rest)?, recipient) {
            (ChatLine::Private { speaker_name, text }, Some(recipient_name)) => {
                ChatLineBody::PrivateEcho(PrivateEcho {
                    speaker_name,
                    recipient_name,
                    text,
                })
            }
            (line, None) => ChatLineBody::Line(line),
            (_, Some(_)) => return Err(ChatWireError::Malformed),
        }
    };
    Ok(ExtendedChatLine {
        body,
        line_sequence: sequence,
    })
}

/// The fields of a kind 8 line other than 12..=14: the kind, the name and the channel only.
fn decode_notice(input: &[u8], kind: ChannelNoticeKind) -> WireResult<ChannelNotice> {
    let (mut name, mut channel) = (None, None);
    read_fields(input, |field, wire, input, cursor| match (field, wire) {
        (1, 0) => read_uint32(input, cursor).map(drop),
        (4, 2) => set_once(&mut name, read_text(input, cursor, MAX_CHAT_NAME_BYTES)?),
        (11, 0) => {
            let channel_id = u16::try_from(read_uint32(input, cursor)?)
                .ok()
                .filter(|channel_id| *channel_id != 0)
                .ok_or(ChatWireError::Malformed)?;
            set_once(&mut channel, channel_id)
        }
        _ => Err(ChatWireError::Malformed),
    })?;
    Ok(ChannelNotice {
        channel_id: channel.ok_or(ChatWireError::Malformed)?,
        kind,
        name: name.ok_or(ChatWireError::Malformed)?,
    })
}

#[cfg(test)]
#[path = "line_extended_tests.rs"]
mod tests;
