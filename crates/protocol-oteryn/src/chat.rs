//! Chat intent, result, room set and line typed payloads (CHAT-1b-1).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/chat_v1.proto`, from CHAT-0 §7
//! (`CHAT0-LOCAL-PRIVATE-WORLD-ROOMS-V1`). Capability 7 `CHAT_V1`, command type 13 `CHAT_INTENT`
//! and state domain 12 `CHAT` are registered in `PROTOCOL_OTERYN_V1_REGISTRY.json`, leased to
//! CHAT-1 by the #162 control plane. The server does not offer the capability before CHAT-1b-2
//! composes it.
//!
//! Decoding is strict, and encoding refuses the same values as a server fault before any byte is
//! emitted: zero or unknown enums, an intent with no or several variants, empty or oversized text
//! or names, invalid UTF-8, a wait outside `1..=`[`MAX_CHAT_WAIT_SECONDS`] with `MUTED` or
//! `EXHAUSTED` or any wait with another disposition, room bits outside the four rooms, a floor
//! outside `0..=15`, a line field that does not belong to its kind, and unknown or repeated fields
//! all fail closed. Trimming and control characters are the server's text rule (CHAT-0 §4).

pub use crate::charm_wire::CyclopediaWireError as ChatWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, push_varint_field, read_bytes,
    read_uint32, read_varint, set_once,
};
use crate::world_spatial::{ActorPosition, decode_position, encode_position};
use crate::world_spatial_entities::ENTITY_IDENTITY_BYTES;

/// Registered capability `CHAT_V1`: command type 13, state domain 12.
pub const CAPABILITY_CHAT_V1: u32 = 7;
/// Registered command type `CHAT_INTENT` (capability 7).
pub const COMMAND_TYPE_CHAT_INTENT: u32 = 13;
/// Registered state domain `CHAT` (capability 7).
pub const STATE_DOMAIN_CHAT: u32 = 12;
/// Snapshot type 1: the open-room set ([`ChatRoomsV1`](encode_chat_rooms)).
pub const SNAPSHOT_TYPE_CHAT_V1: u32 = 1;
/// Delta type 1: one line.
pub const DELTA_TYPE_CHAT_LINE_V1: u32 = 1;
/// Delta type 2: the open-room set, replaced whole.
pub const DELTA_TYPE_CHAT_ROOMS_V1: u32 = 2;

/// `CHAT0-RL-01`: at most 1,020 bytes of text (255 four-byte scalars).
pub const MAX_CHAT_TEXT_BYTES: usize = 1_020;
/// A character name on the chat wire, as in the CHAT-0 §5 relay payload.
pub const MAX_CHAT_NAME_BYTES: usize = 120;
/// The longest wait a result reports: a mute of 5 s × 16² (`CHAT0-RL-04`).
pub const MAX_CHAT_WAIT_SECONDS: u32 = 1_280;
/// `CHAT0-RL-07`: every room can be open at once.
pub const MAX_CHAT_OPEN_ROOMS: usize = 4;
/// A private message: oneof tag 1 + length 2, name 1 + 1 + 120, text 1 + 2 + 1,020.
pub const MAX_CHAT_INTENT_BYTES: usize = 1_148;
/// Disposition 1 + 1 and wait 1 + 2 = 5 bytes, with slack.
pub const MAX_CHAT_RESULT_BYTES: usize = 8;
/// One `open_rooms` field of 1 + 1 bytes, with slack.
pub const MAX_CHAT_ROOMS_BYTES: usize = 4;
/// A local line: kind 2, identity 18, generation 11, name 122, mode 2, text 1,023, position 16.
pub const MAX_CHAT_LINE_BYTES: usize = 1_194;

const MAX_FLOOR: i16 = 15;

/// `ChatSpeechMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatSpeechMode {
    Say = 1,
    Whisper = 2,
    Yell = 3,
}

impl ChatSpeechMode {
    fn from_wire(value: u32) -> WireResult<Self> {
        match value {
            1 => Ok(Self::Say),
            2 => Ok(Self::Whisper),
            3 => Ok(Self::Yell),
            _ => Err(ChatWireError::Malformed),
        }
    }
}

/// `ChatRoom`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChatRoom {
    World = 1,
    English = 2,
    Help = 3,
    Advertising = 4,
}

impl ChatRoom {
    pub const ALL: [Self; MAX_CHAT_OPEN_ROOMS] =
        [Self::World, Self::English, Self::Help, Self::Advertising];

    fn from_wire(value: u32) -> WireResult<Self> {
        Self::ALL
            .into_iter()
            .find(|room| *room as u32 == value)
            .ok_or(ChatWireError::Malformed)
    }

    const fn bit(self) -> u8 {
        1 << (self as u8 - 1)
    }
}

/// `ChatIntentV1`: exactly one intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatIntent {
    Say {
        mode: ChatSpeechMode,
        text: String,
    },
    Private {
        recipient_name: String,
        text: String,
    },
    Room {
        room: ChatRoom,
        text: String,
    },
    OpenRoom(ChatRoom),
    CloseRoom(ChatRoom),
}

/// `ChatDisposition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatDisposition {
    Ok = 1,
    Muted = 2,
    Exhausted = 3,
    LevelTooLow = 4,
    NotOnline = 5,
    NoVocation = 6,
    RoomNotOpen = 7,
    ChatUnavailable = 8,
    Rejected = 9,
}

impl ChatDisposition {
    const ALL: [Self; 9] = [
        Self::Ok,
        Self::Muted,
        Self::Exhausted,
        Self::LevelTooLow,
        Self::NotOnline,
        Self::NoVocation,
        Self::RoomNotOpen,
        Self::ChatUnavailable,
        Self::Rejected,
    ];

    const fn waits(self) -> bool {
        matches!(self, Self::Muted | Self::Exhausted)
    }
}

/// `ChatIntentResultV1`: `wait_seconds` is `1..=`[`MAX_CHAT_WAIT_SECONDS`] with `Muted` and
/// `Exhausted` and 0 otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatIntentResult {
    pub disposition: ChatDisposition,
    pub wait_seconds: u32,
}

/// `ChatRoomsV1`: the rooms a session has open.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChatRoomSet(u8);

impl ChatRoomSet {
    const ALL_BITS: u8 = 0b1111;

    pub fn contains(self, room: ChatRoom) -> bool {
        self.0 & room.bit() != 0
    }

    pub fn insert(&mut self, room: ChatRoom) {
        self.0 |= room.bit();
    }

    pub fn remove(&mut self, room: ChatRoom) {
        self.0 &= !room.bit();
    }
}

/// The spatial identity of a local speaker, as in the `WORLD_SPATIAL_ENTITIES` entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatSpeaker {
    pub identity: [u8; ENTITY_IDENTITY_BYTES],
    pub generation: u64,
}

/// `ChatLineV1`: one line of the session's chat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatLine {
    Local {
        speaker: ChatSpeaker,
        speaker_name: String,
        mode: ChatSpeechMode,
        text: String,
        position: ActorPosition,
    },
    Private {
        speaker_name: String,
        text: String,
    },
    Room {
        room: ChatRoom,
        speaker_name: String,
        text: String,
    },
    /// Lines were dropped by the session's egress bound (`CHAT0-RL-11`).
    Dropped,
}

fn check_text(text: &str, maximum: usize) -> WireResult<()> {
    if text.is_empty() {
        return Err(ChatWireError::Malformed);
    }
    if text.len() > maximum {
        return Err(ChatWireError::LimitExceeded);
    }
    Ok(())
}

fn read_text(input: &[u8], cursor: &mut usize, maximum: usize) -> WireResult<String> {
    let bytes = read_bytes(input, cursor)?;
    let text = std::str::from_utf8(bytes).map_err(|_| ChatWireError::Malformed)?;
    check_text(text, maximum)?;
    Ok(text.to_owned())
}

fn check_position(position: &ActorPosition) -> WireResult<()> {
    if (0..=MAX_FLOOR).contains(&position.floor) {
        Ok(())
    } else {
        Err(ChatWireError::Malformed)
    }
}

/// Reads the fields of a message with a key filter: `field(number, wire_type, input, cursor)`
/// consumes one value or refuses the key.
fn read_fields(
    input: &[u8],
    mut field: impl FnMut(u64, u64, &[u8], &mut usize) -> WireResult<()>,
) -> WireResult<()> {
    let mut cursor = 0;
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        field(key >> 3, key & 0x07, input, &mut cursor)?;
    }
    Ok(())
}

fn encode_text_message(first: &[u8], text: &str) -> Vec<u8> {
    let mut output = Vec::with_capacity(first.len() + text.len() + 3);
    output.extend_from_slice(first);
    push_message_field(&mut output, 2, text.as_bytes());
    output
}

/// Encodes a `ChatIntentV1` (the client side).
pub fn encode_chat_intent(intent: &ChatIntent) -> WireResult<Vec<u8>> {
    let mut inner = Vec::new();
    let field = match intent {
        ChatIntent::Say { mode, text } => {
            check_text(text, MAX_CHAT_TEXT_BYTES)?;
            push_varint_field(&mut inner, 1, *mode as u64);
            inner = encode_text_message(&inner, text);
            1
        }
        ChatIntent::Private {
            recipient_name,
            text,
        } => {
            check_text(recipient_name, MAX_CHAT_NAME_BYTES)?;
            check_text(text, MAX_CHAT_TEXT_BYTES)?;
            push_message_field(&mut inner, 1, recipient_name.as_bytes());
            inner = encode_text_message(&inner, text);
            2
        }
        ChatIntent::Room { room, text } => {
            check_text(text, MAX_CHAT_TEXT_BYTES)?;
            push_varint_field(&mut inner, 1, *room as u64);
            inner = encode_text_message(&inner, text);
            3
        }
        ChatIntent::OpenRoom(room) | ChatIntent::CloseRoom(room) => {
            push_varint_field(&mut inner, 1, *room as u64);
            if matches!(intent, ChatIntent::OpenRoom(_)) {
                4
            } else {
                5
            }
        }
    };
    let mut output = Vec::with_capacity(inner.len() + 3);
    push_message_field(&mut output, field, &inner);
    Ok(output)
}

/// Decodes a `ChatIntentV1`; the server answers `REJECTED` to any error.
pub fn decode_chat_intent(payload: &[u8]) -> WireResult<ChatIntent> {
    if payload.len() > MAX_CHAT_INTENT_BYTES {
        return Err(ChatWireError::LimitExceeded);
    }
    let mut intent = None;
    read_fields(payload, |field, wire, input, cursor| {
        if wire != 2 || !(1..=5).contains(&field) {
            return Err(ChatWireError::Malformed);
        }
        let inner = read_bytes(input, cursor)?;
        set_once(&mut intent, decode_intent_variant(field, inner)?)
    })?;
    intent.ok_or(ChatWireError::Malformed)
}

fn decode_intent_variant(field: u64, inner: &[u8]) -> WireResult<ChatIntent> {
    let (mut first, mut text) = (None, None);
    read_fields(inner, |number, wire, input, cursor| match (number, wire) {
        (1, 0) if field != 2 => set_once(&mut first, FirstField::Enum(read_uint32(input, cursor)?)),
        (1, 2) if field == 2 => set_once(
            &mut first,
            FirstField::Name(read_text(input, cursor, MAX_CHAT_NAME_BYTES)?),
        ),
        (2, 2) if field <= 3 => set_once(&mut text, read_text(input, cursor, MAX_CHAT_TEXT_BYTES)?),
        _ => Err(ChatWireError::Malformed),
    })?;
    let enum_value = |first: Option<FirstField>| match first {
        Some(FirstField::Enum(value)) => Ok(value),
        _ => Err(ChatWireError::Malformed),
    };
    let text = || text.clone().ok_or(ChatWireError::Malformed);
    Ok(match field {
        1 => ChatIntent::Say {
            mode: ChatSpeechMode::from_wire(enum_value(first)?)?,
            text: text()?,
        },
        2 => match first {
            Some(FirstField::Name(recipient_name)) => ChatIntent::Private {
                recipient_name,
                text: text()?,
            },
            _ => return Err(ChatWireError::Malformed),
        },
        3 => ChatIntent::Room {
            room: ChatRoom::from_wire(enum_value(first)?)?,
            text: text()?,
        },
        4 => ChatIntent::OpenRoom(ChatRoom::from_wire(enum_value(first)?)?),
        _ => ChatIntent::CloseRoom(ChatRoom::from_wire(enum_value(first)?)?),
    })
}

enum FirstField {
    Enum(u32),
    Name(String),
}

fn check_result(result: &ChatIntentResult) -> WireResult<()> {
    let waits = result.disposition.waits();
    if result.wait_seconds > MAX_CHAT_WAIT_SECONDS {
        return Err(ChatWireError::LimitExceeded);
    }
    if waits != (result.wait_seconds != 0) {
        return Err(ChatWireError::Malformed);
    }
    Ok(())
}

/// Encodes a `ChatIntentResultV1` (the server side).
pub fn encode_chat_intent_result(result: &ChatIntentResult) -> WireResult<Vec<u8>> {
    check_result(result)?;
    let mut output = Vec::with_capacity(MAX_CHAT_RESULT_BYTES);
    push_varint_field(&mut output, 1, result.disposition as u64);
    push_nonzero_varint_field(&mut output, 2, u64::from(result.wait_seconds));
    Ok(output)
}

pub fn decode_chat_intent_result(payload: &[u8]) -> WireResult<ChatIntentResult> {
    if payload.len() > MAX_CHAT_RESULT_BYTES {
        return Err(ChatWireError::LimitExceeded);
    }
    let [disposition, wait_seconds] = crate::charm_wire::read_uint32_fields::<2>(payload)?;
    let result = ChatIntentResult {
        disposition: ChatDisposition::ALL
            .into_iter()
            .find(|value| *value as u32 == disposition)
            .ok_or(ChatWireError::Malformed)?,
        wait_seconds,
    };
    check_result(&result)?;
    Ok(result)
}

/// Encodes the snapshot (type 1) or rooms delta (type 2) payload.
pub fn encode_chat_rooms(rooms: ChatRoomSet) -> Vec<u8> {
    let mut output = Vec::with_capacity(MAX_CHAT_ROOMS_BYTES);
    push_nonzero_varint_field(&mut output, 1, u64::from(rooms.0));
    output
}

pub fn decode_chat_rooms(payload: &[u8]) -> WireResult<ChatRoomSet> {
    if payload.len() > MAX_CHAT_ROOMS_BYTES {
        return Err(ChatWireError::LimitExceeded);
    }
    let [bits] = crate::charm_wire::read_uint32_fields::<1>(payload)?;
    let bits = u8::try_from(bits)
        .ok()
        .filter(|bits| bits & !ChatRoomSet::ALL_BITS == 0)
        .ok_or(ChatWireError::Malformed)?;
    Ok(ChatRoomSet(bits))
}

/// Encodes a line delta (type 1). A line outside the bounds is a server fault.
pub fn encode_chat_line(line: &ChatLine) -> WireResult<Vec<u8>> {
    let mut output = Vec::with_capacity(MAX_CHAT_LINE_BYTES);
    match line {
        ChatLine::Local {
            speaker,
            speaker_name,
            mode,
            text,
            position,
        } => {
            check_text(speaker_name, MAX_CHAT_NAME_BYTES)?;
            check_text(text, MAX_CHAT_TEXT_BYTES)?;
            check_position(position)?;
            push_varint_field(&mut output, 1, 1);
            push_message_field(&mut output, 2, &speaker.identity);
            push_nonzero_varint_field(&mut output, 3, speaker.generation);
            push_message_field(&mut output, 4, speaker_name.as_bytes());
            push_varint_field(&mut output, 5, *mode as u64);
            push_message_field(&mut output, 7, text.as_bytes());
            push_message_field(&mut output, 8, &encode_position(position));
        }
        ChatLine::Private { speaker_name, text } => {
            check_text(speaker_name, MAX_CHAT_NAME_BYTES)?;
            check_text(text, MAX_CHAT_TEXT_BYTES)?;
            push_varint_field(&mut output, 1, 2);
            push_message_field(&mut output, 4, speaker_name.as_bytes());
            push_message_field(&mut output, 7, text.as_bytes());
        }
        ChatLine::Room {
            room,
            speaker_name,
            text,
        } => {
            check_text(speaker_name, MAX_CHAT_NAME_BYTES)?;
            check_text(text, MAX_CHAT_TEXT_BYTES)?;
            push_varint_field(&mut output, 1, 3);
            push_message_field(&mut output, 4, speaker_name.as_bytes());
            push_varint_field(&mut output, 6, *room as u64);
            push_message_field(&mut output, 7, text.as_bytes());
        }
        ChatLine::Dropped => push_varint_field(&mut output, 1, 4),
    }
    Ok(output)
}

#[derive(Default)]
struct LineFields {
    kind: Option<u32>,
    identity: Option<[u8; ENTITY_IDENTITY_BYTES]>,
    generation: Option<u64>,
    name: Option<String>,
    mode: Option<u32>,
    room: Option<u32>,
    text: Option<String>,
    position: Option<ActorPosition>,
}

pub fn decode_chat_line(payload: &[u8]) -> WireResult<ChatLine> {
    if payload.len() > MAX_CHAT_LINE_BYTES {
        return Err(ChatWireError::LimitExceeded);
    }
    let mut f = LineFields::default();
    read_fields(payload, |number, wire, input, cursor| {
        match (number, wire) {
            (1, 0) => set_once(&mut f.kind, read_uint32(input, cursor)?),
            (2, 2) => {
                let identity = read_bytes(input, cursor)?
                    .try_into()
                    .map_err(|_| ChatWireError::Malformed)?;
                set_once(&mut f.identity, identity)
            }
            (3, 0) => set_once(&mut f.generation, read_varint(input, cursor)?),
            (4, 2) => set_once(&mut f.name, read_text(input, cursor, MAX_CHAT_NAME_BYTES)?),
            (5, 0) => set_once(&mut f.mode, read_uint32(input, cursor)?),
            (6, 0) => set_once(&mut f.room, read_uint32(input, cursor)?),
            (7, 2) => set_once(&mut f.text, read_text(input, cursor, MAX_CHAT_TEXT_BYTES)?),
            (8, 2) => {
                let position = decode_position(read_bytes(input, cursor)?)
                    .map_err(|_| ChatWireError::Malformed)?;
                check_position(&position)?;
                set_once(&mut f.position, position)
            }
            _ => Err(ChatWireError::Malformed),
        }
    })?;
    let malformed = || ChatWireError::Malformed;
    match f.kind {
        Some(1) if f.room.is_none() => Ok(ChatLine::Local {
            speaker: ChatSpeaker {
                identity: f.identity.ok_or_else(malformed)?,
                generation: f.generation.unwrap_or(0),
            },
            speaker_name: f.name.ok_or_else(malformed)?,
            mode: ChatSpeechMode::from_wire(f.mode.ok_or_else(malformed)?)?,
            text: f.text.ok_or_else(malformed)?,
            position: f.position.ok_or_else(malformed)?,
        }),
        Some(2) if no_spatial(&f) && f.room.is_none() => Ok(ChatLine::Private {
            speaker_name: f.name.ok_or_else(malformed)?,
            text: f.text.ok_or_else(malformed)?,
        }),
        Some(3) if no_spatial(&f) => Ok(ChatLine::Room {
            room: ChatRoom::from_wire(f.room.ok_or_else(malformed)?)?,
            speaker_name: f.name.ok_or_else(malformed)?,
            text: f.text.ok_or_else(malformed)?,
        }),
        Some(4) if no_spatial(&f) && f.room.is_none() && f.name.is_none() && f.text.is_none() => {
            Ok(ChatLine::Dropped)
        }
        _ => Err(malformed()),
    }
}

/// No local-only field: identity, generation, mode or position.
fn no_spatial(f: &LineFields) -> bool {
    f.identity.is_none() && f.generation.is_none() && f.mode.is_none() && f.position.is_none()
}

#[cfg(test)]
#[path = "chat_tests.rs"]
mod tests;
