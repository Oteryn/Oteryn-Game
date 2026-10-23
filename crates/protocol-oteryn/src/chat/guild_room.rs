//! Negotiated CHAT_V1 guild extension. Public rooms retain their four-bit encoding.
use super::*;

pub const CAPABILITY_CHAT_GUILD_ROOM_V1: u32 = 31;
pub const SNAPSHOT_TYPE_GUILD_CHAT_V1: u32 = 2;
pub const MAX_GUILD_CHAT_SNAPSHOT_BYTES: usize = 1_049;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildChatLine {
    pub guild_id: [u8; 16],
    pub rank: u8,
    pub speaker_name: String,
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum GuildRoomState {
    #[default]
    Absent,
    Unavailable,
    Member {
        guild_id: [u8; 16],
        rank: u8,
        motd: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildChatSnapshot {
    pub rooms: ChatRoomSet,
    pub guild: GuildRoomState,
}

fn check_destination(guild_id: &[u8; 16], rank: u8) -> WireResult<()> {
    crate::CharacterId::decode(guild_id).map_err(|_| ChatWireError::Malformed)?;
    if !(1..=20).contains(&rank) {
        return Err(ChatWireError::Malformed);
    }
    Ok(())
}

pub(super) fn encode_intent(text: &str) -> WireResult<Vec<u8>> {
    check_text(text, MAX_CHAT_TEXT_BYTES)?;
    let mut inner = Vec::new();
    push_message_field(&mut inner, 1, text.as_bytes());
    let mut out = Vec::new();
    push_message_field(&mut out, 6, &inner);
    Ok(out)
}

pub(super) fn decode_intent(input: &[u8]) -> WireResult<ChatIntent> {
    let mut text = None;
    read_fields(input, |field, wire, input, cursor| match (field, wire) {
        (1, 2) => set_once(&mut text, read_text(input, cursor, MAX_CHAT_TEXT_BYTES)?),
        _ => Err(ChatWireError::Malformed),
    })?;
    Ok(ChatIntent::Guild {
        text: text.ok_or(ChatWireError::Malformed)?,
    })
}

pub(super) fn encode_line(line: &GuildChatLine) -> WireResult<Vec<u8>> {
    check_destination(&line.guild_id, line.rank)?;
    check_text(&line.speaker_name, MAX_CHAT_NAME_BYTES)?;
    check_text(&line.text, MAX_CHAT_TEXT_BYTES)?;
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, 5);
    push_message_field(&mut out, 4, line.speaker_name.as_bytes());
    push_message_field(&mut out, 7, line.text.as_bytes());
    push_message_field(&mut out, 9, &line.guild_id);
    push_varint_field(&mut out, 10, u64::from(line.rank));
    Ok(out)
}

pub(super) fn decode_line_if_guild(input: &[u8]) -> WireResult<Option<GuildChatLine>> {
    // Inspect only the kind, then let the owning codec reject every alien/repeated field.
    let mut kind = None;
    read_fields(input, |field, wire, input, cursor| {
        if (field, wire) == (1, 0) {
            set_once(&mut kind, read_uint32(input, cursor)?)
        } else {
            match wire {
                0 => {
                    read_varint(input, cursor)?;
                }
                2 => {
                    read_bytes(input, cursor)?;
                }
                _ => return Err(ChatWireError::Malformed),
            }
            Ok(())
        }
    })?;
    if kind != Some(5) {
        return Ok(None);
    }
    let (mut guild, mut rank, mut name, mut text) = (None, None, None, None);
    read_fields(input, |field, wire, input, cursor| match (field, wire) {
        (1, 0) => {
            read_uint32(input, cursor)?;
            Ok(())
        }
        (4, 2) => set_once(&mut name, read_text(input, cursor, MAX_CHAT_NAME_BYTES)?),
        (7, 2) => set_once(&mut text, read_text(input, cursor, MAX_CHAT_TEXT_BYTES)?),
        (9, 2) => set_once(
            &mut guild,
            read_bytes(input, cursor)?
                .try_into()
                .map_err(|_| ChatWireError::Malformed)?,
        ),
        (10, 0) => set_once(
            &mut rank,
            u8::try_from(read_uint32(input, cursor)?).map_err(|_| ChatWireError::Malformed)?,
        ),
        _ => Err(ChatWireError::Malformed),
    })?;
    let line = GuildChatLine {
        guild_id: guild.ok_or(ChatWireError::Malformed)?,
        rank: rank.ok_or(ChatWireError::Malformed)?,
        speaker_name: name.ok_or(ChatWireError::Malformed)?,
        text: text.ok_or(ChatWireError::Malformed)?,
    };
    check_destination(&line.guild_id, line.rank)?;
    Ok(Some(line))
}

#[path = "guild_snapshot.rs"]
mod guild_snapshot;
pub use guild_snapshot::{decode_snapshot, encode_snapshot};

#[cfg(test)]
#[path = "guild_room_tests.rs"]
mod tests;
