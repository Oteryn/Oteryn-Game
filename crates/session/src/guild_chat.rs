//! Negotiated CHAT_V1 room extensions. The client holds no role or membership authority.
use super::*;
use oteryn_protocol_oteryn::chat::guild_room::{self, CAPABILITY_CHAT_GUILD_ROOM_V1};
use oteryn_protocol_oteryn::chat::private_channel::CAPABILITY_CHAT_PRIVATE_CHANNEL_V1;
use oteryn_protocol_oteryn::chat::public_channels::CAPABILITY_CHAT_PUBLIC_CHANNELS_V1;

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
    check_rooms(rooms, selected)?;
    Ok(ChatLog {
        rooms,
        guild,
        private_channels: None,
        lines: VecDeque::new(),
    })
}

pub(super) fn check_intent(intent: &ChatIntent, selected: &[u32]) -> Result<(), SessionError> {
    if matches!(intent, ChatIntent::PrivateChannel(_))
        && !selected.contains(&CAPABILITY_CHAT_PRIVATE_CHANNEL_V1)
    {
        return Err(SessionError::CapabilityNotSelected {
            capability: CAPABILITY_CHAT_PRIVATE_CHANNEL_V1,
        });
    }
    if let ChatIntent::OpenRoom(room)
    | ChatIntent::CloseRoom(room)
    | ChatIntent::Room { room, .. } = intent
    {
        check_room(*room, selected)?;
    }
    if matches!(intent, ChatIntent::Guild { .. })
        && !selected.contains(&CAPABILITY_CHAT_GUILD_ROOM_V1)
    {
        return Err(SessionError::CapabilityNotSelected {
            capability: CAPABILITY_CHAT_GUILD_ROOM_V1,
        });
    }
    if matches!(intent, ChatIntent::Party { .. })
        && !selected
            .contains(&oteryn_protocol_oteryn::chat::party_room::CAPABILITY_CHAT_PARTY_ROOM_V1)
    {
        return Err(SessionError::CapabilityNotSelected {
            capability: oteryn_protocol_oteryn::chat::party_room::CAPABILITY_CHAT_PARTY_ROOM_V1,
        });
    }
    Ok(())
}

pub(super) fn decode_line(payload: &[u8], selected: &[u32]) -> Result<ChatLine, SessionError> {
    let line = chat::decode_chat_line(payload)?;
    if matches!(line, ChatLine::PrivateChannel(_))
        && !selected.contains(&CAPABILITY_CHAT_PRIVATE_CHANNEL_V1)
    {
        return Err(SessionError::CapabilityNotSelected {
            capability: CAPABILITY_CHAT_PRIVATE_CHANNEL_V1,
        });
    }
    if let ChatLine::Room { room, .. } = &line {
        check_room(*room, selected)?;
    }
    if matches!(line, ChatLine::Guild(_)) && !selected.contains(&CAPABILITY_CHAT_GUILD_ROOM_V1) {
        return Err(SessionError::CapabilityNotSelected {
            capability: CAPABILITY_CHAT_GUILD_ROOM_V1,
        });
    }
    if matches!(line, ChatLine::Party { .. })
        && !selected
            .contains(&oteryn_protocol_oteryn::chat::party_room::CAPABILITY_CHAT_PARTY_ROOM_V1)
    {
        return Err(SessionError::CapabilityNotSelected {
            capability: oteryn_protocol_oteryn::chat::party_room::CAPABILITY_CHAT_PARTY_ROOM_V1,
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
    if selected.contains(&CAPABILITY_CHAT_PRIVATE_CHANNEL_V1)
        && !selected.contains(&CAPABILITY_CHAT_V1)
    {
        return Err(SessionError::CapabilityNotSelected {
            capability: CAPABILITY_CHAT_V1,
        });
    }
    if selected.contains(&CAPABILITY_CHAT_PUBLIC_CHANNELS_V1) {
        for capability in [CAPABILITY_CHAT_V1, CAPABILITY_CHAT_PRIVATE_CHANNEL_V1] {
            if !selected.contains(&capability) {
                return Err(SessionError::CapabilityNotSelected { capability });
            }
        }
    }
    if selected.contains(&oteryn_protocol_oteryn::chat::party_room::CAPABILITY_CHAT_PARTY_ROOM_V1) {
        for capability in [
            CAPABILITY_CHAT_V1,
            oteryn_protocol_oteryn::party::CAPABILITY_PARTY_V1,
        ] {
            if !selected.contains(&capability) {
                return Err(SessionError::CapabilityNotSelected { capability });
            }
        }
    }
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
    if disposition == ChatDisposition::NotInvited
        && !selected.contains(&CAPABILITY_CHAT_PRIVATE_CHANNEL_V1)
    {
        return Err(SessionError::CapabilityNotSelected {
            capability: CAPABILITY_CHAT_PRIVATE_CHANNEL_V1,
        });
    }
    if disposition == ChatDisposition::NotAllowed
        && !selected.contains(&CAPABILITY_CHAT_GUILD_ROOM_V1)
        && !selected.contains(&CAPABILITY_CHAT_PRIVATE_CHANNEL_V1)
        && !selected
            .contains(&oteryn_protocol_oteryn::chat::party_room::CAPABILITY_CHAT_PARTY_ROOM_V1)
    {
        return Err(SessionError::CapabilityNotSelected {
            capability: CAPABILITY_CHAT_GUILD_ROOM_V1,
        });
    }
    Ok(())
}

fn check_room(room: ChatRoom, selected: &[u32]) -> Result<(), SessionError> {
    if room.requires_public_channels() && !selected.contains(&CAPABILITY_CHAT_PUBLIC_CHANNELS_V1) {
        return Err(SessionError::CapabilityNotSelected {
            capability: CAPABILITY_CHAT_PUBLIC_CHANNELS_V1,
        });
    }
    Ok(())
}

/// Room snapshots, replacement deltas and private-channel lists use the same
/// capability boundary before changing any session state.
pub(super) fn check_rooms(rooms: ChatRoomSet, selected: &[u32]) -> Result<(), SessionError> {
    for room in ChatRoom::ALL {
        if rooms.contains(room) {
            check_room(room, selected)?;
        }
    }
    Ok(())
}

pub(super) fn check_result_for_intent(
    disposition: ChatDisposition,
    selected: &[u32],
    intent: &ChatIntent,
) -> Result<(), SessionError> {
    // Public-room role/level gates also apply to the four legacy rooms. Their
    // NOT_ALLOWED reply is valid without Guild/Party or the four-room extension.
    if disposition == ChatDisposition::NotAllowed
        && matches!(
            intent,
            ChatIntent::OpenRoom(_) | ChatIntent::CloseRoom(_) | ChatIntent::Room { .. }
        )
    {
        return check_intent(intent, selected);
    }
    check_result(disposition, selected)
}

#[cfg(test)]
#[path = "public_channels_tests.rs"]
mod public_channels_tests;
