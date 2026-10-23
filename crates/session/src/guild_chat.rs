//! CHAT_V1 guild extension routing. No guild roster or authority is held by the client.
use super::*;
use oteryn_protocol_oteryn::chat::guild_room::{self, CAPABILITY_CHAT_GUILD_ROOM_V1};

pub(super) fn load_snapshot(
    snapshot_type: u32,
    payload: &[u8],
    selected: &[u32],
) -> Result<ChatLog, SessionError> {
    let (rooms, guild) = match snapshot_type {
        guild_room::SNAPSHOT_TYPE_GUILD_CHAT_V1
            if selected.contains(&CAPABILITY_CHAT_GUILD_ROOM_V1) =>
        {
            let snapshot = guild_room::decode_snapshot(payload)?;
            (snapshot.rooms, snapshot.guild)
        }
        chat::SNAPSHOT_TYPE_CHAT_V1 if !selected.contains(&CAPABILITY_CHAT_GUILD_ROOM_V1) => (
            chat::decode_chat_rooms(payload)?,
            guild_room::GuildRoomState::Absent,
        ),
        _ => {
            return Err(SessionError::UnregisteredSnapshotType {
                domain_id: STATE_DOMAIN_CHAT,
                snapshot_type,
            });
        }
    };
    Ok(ChatLog {
        rooms,
        guild,
        lines: VecDeque::new(),
    })
}

pub(super) fn check_intent(intent: &ChatIntent, selected: &[u32]) -> Result<(), SessionError> {
    if matches!(intent, ChatIntent::Guild { .. })
        && !selected.contains(&CAPABILITY_CHAT_GUILD_ROOM_V1)
    {
        return Err(SessionError::CapabilityNotSelected {
            capability: CAPABILITY_CHAT_GUILD_ROOM_V1,
        });
    }
    Ok(())
}

pub(super) fn decode_line(payload: &[u8], selected: &[u32]) -> Result<ChatLine, SessionError> {
    let line = chat::decode_chat_line(payload)?;
    if matches!(line, ChatLine::Guild(_)) && !selected.contains(&CAPABILITY_CHAT_GUILD_ROOM_V1) {
        return Err(SessionError::CapabilityNotSelected {
            capability: CAPABILITY_CHAT_GUILD_ROOM_V1,
        });
    }
    Ok(line)
}

impl ChatLog {
    #[must_use]
    pub const fn guild_room(&self) -> &guild_room::GuildRoomState {
        &self.guild
    }
}

pub(super) fn check_selection(selected: &[u32]) -> Result<(), SessionError> {
    if selected.contains(&CAPABILITY_CHAT_GUILD_ROOM_V1) && !selected.contains(&CAPABILITY_CHAT_V1)
    {
        return Err(SessionError::CapabilityNotSelected {
            capability: CAPABILITY_CHAT_V1,
        });
    }
    Ok(())
}

pub(super) fn check_result(
    disposition: ChatDisposition,
    selected: &[u32],
) -> Result<(), SessionError> {
    if disposition == ChatDisposition::NotAllowed
        && !selected.contains(&CAPABILITY_CHAT_GUILD_ROOM_V1)
    {
        return Err(SessionError::CapabilityNotSelected {
            capability: CAPABILITY_CHAT_GUILD_ROOM_V1,
        });
    }
    Ok(())
}
