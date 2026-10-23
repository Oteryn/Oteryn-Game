use super::*;

pub fn encode_snapshot(snapshot: &GuildChatSnapshot) -> WireResult<Vec<u8>> {
    let mut out = Vec::new();
    push_message_field(&mut out, 1, &encode_chat_rooms(snapshot.rooms));
    match &snapshot.guild {
        GuildRoomState::Absent => push_varint_field(&mut out, 2, 1),
        GuildRoomState::Unavailable => push_varint_field(&mut out, 2, 2),
        GuildRoomState::Member {
            guild_id,
            rank,
            motd,
        } => {
            check_destination(guild_id, *rank)?;
            if motd.len() > MAX_CHAT_TEXT_BYTES || motd.chars().count() > 255 {
                return Err(ChatWireError::LimitExceeded);
            }
            push_varint_field(&mut out, 2, 3);
            push_message_field(&mut out, 3, guild_id);
            push_varint_field(&mut out, 4, u64::from(*rank));
            push_message_field(&mut out, 5, motd.as_bytes());
        }
    }
    Ok(out)
}

pub fn decode_snapshot(input: &[u8]) -> WireResult<GuildChatSnapshot> {
    if input.len() > MAX_GUILD_CHAT_SNAPSHOT_BYTES {
        return Err(ChatWireError::LimitExceeded);
    }
    let (mut rooms, mut kind, mut guild, mut rank, mut motd) = (None, None, None, None, None);
    read_fields(input, |field, wire, input, cursor| match (field, wire) {
        (1, 2) => set_once(&mut rooms, decode_chat_rooms(read_bytes(input, cursor)?)?),
        (2, 0) => set_once(&mut kind, read_uint32(input, cursor)?),
        (3, 2) => set_once(
            &mut guild,
            read_bytes(input, cursor)?
                .try_into()
                .map_err(|_| ChatWireError::Malformed)?,
        ),
        (4, 0) => set_once(
            &mut rank,
            u8::try_from(read_uint32(input, cursor)?).map_err(|_| ChatWireError::Malformed)?,
        ),
        (5, 2) => {
            let bytes = read_bytes(input, cursor)?;
            if bytes.len() > MAX_CHAT_TEXT_BYTES {
                return Err(ChatWireError::LimitExceeded);
            }
            let text = std::str::from_utf8(bytes).map_err(|_| ChatWireError::Malformed)?;
            if text.chars().count() > 255 {
                return Err(ChatWireError::LimitExceeded);
            }
            set_once(&mut motd, text.to_owned())
        }
        _ => Err(ChatWireError::Malformed),
    })?;
    let state = match kind {
        Some(1 | 2) if guild.is_none() && rank.is_none() && motd.is_none() => {
            if kind == Some(1) {
                GuildRoomState::Absent
            } else {
                GuildRoomState::Unavailable
            }
        }
        Some(3) => {
            let guild_id = guild.ok_or(ChatWireError::Malformed)?;
            let rank = rank.ok_or(ChatWireError::Malformed)?;
            check_destination(&guild_id, rank)?;
            GuildRoomState::Member {
                guild_id,
                rank,
                motd: motd.ok_or(ChatWireError::Malformed)?,
            }
        }
        _ => return Err(ChatWireError::Malformed),
    };
    Ok(GuildChatSnapshot {
        rooms: rooms.ok_or(ChatWireError::Malformed)?,
        guild: state,
    })
}
