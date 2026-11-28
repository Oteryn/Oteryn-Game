//! Capability-gated extension to the existing CHAT_INTENT command.
use super::*;

pub const CAPABILITY_CHAT_GUILD_GUEST_V1: u32 = 656;
// Existing guild ranks start at 1. A zero here identifies a guest speaker.
pub const GUILD_GUEST_RANK: u8 = 0;
const FIELD: u64 = 15;
const ROOMS_SECTION: u64 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuildGuestIntent {
    Invite { character: [u8; 16] },
    Exclude { character: [u8; 16] },
    Send { guild: [u8; 16], text: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestRoom {
    pub guild: [u8; 16],
    pub name: String,
}

pub const MAX_GUEST_ROOMS: usize = crate::guild::MAX_GUILD_PAGE_ENTRIES;

pub fn encode_intent(intent: &GuildGuestIntent) -> WireResult<Vec<u8>> {
    let mut body = Vec::new();
    match intent {
        GuildGuestIntent::Invite { character } => push_message_field(&mut body, 1, character),
        GuildGuestIntent::Exclude { character } => push_message_field(&mut body, 2, character),
        GuildGuestIntent::Send { guild, text } => {
            let mut send = Vec::new();
            push_message_field(&mut send, 1, guild);
            check_text(text, MAX_CHAT_TEXT_BYTES)?;
            push_message_field(&mut send, 2, text.as_bytes());
            push_message_field(&mut body, 3, &send);
        }
    }
    let mut bytes = Vec::new();
    push_message_field(&mut bytes, FIELD, &body);
    decode_intent(&bytes)?;
    Ok(bytes)
}

pub fn is_extension(bytes: &[u8]) -> bool {
    read_varint(bytes, &mut 0).is_ok_and(|tag| tag >> 3 == FIELD)
}

pub fn decode_intent(bytes: &[u8]) -> WireResult<GuildGuestIntent> {
    if bytes.len() > MAX_CHAT_INTENT_BYTES {
        return Err(ChatWireError::LimitExceeded);
    }
    let mut result = None;
    read_fields(bytes, |field, wire, bytes, cursor| match (field, wire) {
        (FIELD, 2) => set_once(&mut result, decode_body(read_bytes(bytes, cursor)?)?),
        _ => Err(ChatWireError::Malformed),
    })?;
    result.ok_or(ChatWireError::Malformed)
}

fn id(bytes: &[u8]) -> WireResult<[u8; 16]> {
    crate::CharacterId::decode(bytes).map_err(|_| ChatWireError::Malformed)?;
    bytes.try_into().map_err(|_| ChatWireError::Malformed)
}

fn decode_body(body: &[u8]) -> WireResult<GuildGuestIntent> {
    let mut result = None;
    read_fields(body, |field, wire, bytes, cursor| {
        if wire != 2 {
            return Err(ChatWireError::Malformed);
        }
        let value = read_bytes(bytes, cursor)?;
        let intent = match field {
            1 => GuildGuestIntent::Invite {
                character: id(value)?,
            },
            2 => GuildGuestIntent::Exclude {
                character: id(value)?,
            },
            3 => {
                let (mut guild, mut text) = (None, None);
                read_fields(value, |field, wire, bytes, cursor| match (field, wire) {
                    (1, 2) => set_once(&mut guild, id(read_bytes(bytes, cursor)?)?),
                    (2, 2) => set_once(&mut text, read_text(bytes, cursor, MAX_CHAT_TEXT_BYTES)?),
                    _ => Err(ChatWireError::Malformed),
                })?;
                GuildGuestIntent::Send {
                    guild: guild.ok_or(ChatWireError::Malformed)?,
                    text: text.ok_or(ChatWireError::Malformed)?,
                }
            }
            _ => return Err(ChatWireError::Malformed),
        };
        set_once(&mut result, intent)
    })?;
    result.ok_or(ChatWireError::Malformed)
}

pub fn is_rooms_query(bytes: &[u8]) -> bool {
    read_varint(bytes, &mut 0).is_ok_and(|tag| tag >> 3 == ROOMS_SECTION)
}

pub fn encode_rooms_query(after: Option<[u8; 16]>) -> WireResult<Vec<u8>> {
    let mut bytes = Vec::new();
    push_message_field(
        &mut bytes,
        ROOMS_SECTION,
        after.as_ref().map_or(&[], <[u8; 16]>::as_slice),
    );
    decode_rooms_query(&bytes)?;
    Ok(bytes)
}

pub fn decode_rooms_query(bytes: &[u8]) -> WireResult<Option<[u8; 16]>> {
    if bytes.len() > crate::guild::MAX_GUILD_REQUEST_BYTES {
        return Err(ChatWireError::LimitExceeded);
    }
    let body = rooms_section(bytes)?;
    (!body.is_empty()).then(|| id(body)).transpose()
}

fn rooms_section(bytes: &[u8]) -> WireResult<&[u8]> {
    let mut cursor = 0;
    if read_varint(bytes, &mut cursor)? != (ROOMS_SECTION << 3 | 2) {
        return Err(ChatWireError::Malformed);
    }
    let body = read_bytes(bytes, &mut cursor)?;
    if cursor != bytes.len() {
        return Err(ChatWireError::Malformed);
    }
    Ok(body)
}

/// A full page can be continued with its last guild as the next selector.
pub fn encode_rooms(rooms: &[GuestRoom]) -> WireResult<Vec<u8>> {
    if rooms.len() > MAX_GUEST_ROOMS {
        return Err(ChatWireError::LimitExceeded);
    }
    let mut body = Vec::new();
    for room in rooms {
        id(&room.guild)?;
        check_text(&room.name, MAX_CHAT_NAME_BYTES)?;
        let mut entry = Vec::new();
        push_message_field(&mut entry, 1, &room.guild);
        push_message_field(&mut entry, 2, room.name.as_bytes());
        push_message_field(&mut body, 1, &entry);
    }
    let mut bytes = Vec::new();
    push_message_field(&mut bytes, ROOMS_SECTION, &body);
    Ok(bytes)
}

pub fn decode_rooms(bytes: &[u8]) -> WireResult<Vec<GuestRoom>> {
    let mut rooms = Vec::<GuestRoom>::new();
    if bytes.len() > crate::guild::MAX_GUILD_RESULT_BYTES {
        return Err(ChatWireError::LimitExceeded);
    }
    read_fields(rooms_section(bytes)?, |field, wire, bytes, cursor| {
        if (field, wire) != (1, 2) || rooms.len() == MAX_GUEST_ROOMS {
            return Err(ChatWireError::Malformed);
        }
        let (mut guild, mut name) = (None, None);
        read_fields(
            read_bytes(bytes, cursor)?,
            |field, wire, bytes, cursor| match (field, wire) {
                (1, 2) => set_once(&mut guild, id(read_bytes(bytes, cursor)?)?),
                (2, 2) => set_once(&mut name, read_text(bytes, cursor, MAX_CHAT_NAME_BYTES)?),
                _ => Err(ChatWireError::Malformed),
            },
        )?;
        let guild = guild.ok_or(ChatWireError::Malformed)?;
        if rooms.last().is_some_and(|previous| previous.guild >= guild) {
            return Err(ChatWireError::Malformed);
        }
        rooms.push(GuestRoom {
            guild,
            name: name.ok_or(ChatWireError::Malformed)?,
        });
        Ok(())
    })?;
    Ok(rooms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_guild_speech_bytes_decode_after_guest_extension() -> WireResult<()> {
        let bytes = [0x32, 0x04, 0x0a, 0x02, b'h', b'i'];
        assert!(!is_extension(&bytes));
        assert_eq!(
            decode_chat_intent(&bytes)?,
            ChatIntent::Guild { text: "hi".into() }
        );
        let mut guild = [1; 16];
        guild[6] = 0x70;
        guild[8] = 0x80;
        let old_line = [
            &[8, 5, 34, 1, b'A', 58, 2, b'h', b'i', 74, 16][..],
            &guild,
            &[80, 2],
        ]
        .concat();
        let line = ChatLine::Guild(guild_room::GuildChatLine {
            guild_id: guild,
            rank: 2,
            speaker_name: "A".into(),
            text: "hi".into(),
        });
        assert_eq!(decode_chat_line(&old_line)?, line);
        assert_eq!(encode_chat_line(&line)?, old_line);
        let invite = GuildGuestIntent::Invite { character: guild };
        assert_eq!(decode_intent(&encode_intent(&invite)?)?, invite);
        assert_eq!(
            decode_rooms_query(&encode_rooms_query(Some(guild))?)?,
            Some(guild)
        );
        let rooms = [GuestRoom {
            guild,
            name: "Guild".into(),
        }];
        assert_eq!(decode_rooms(&encode_rooms(&rooms)?)?, rooms);
        Ok(())
    }
}
