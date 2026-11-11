//! WU26 private-channel list extensions: the joined channels (capability 494) and their
//! participants (capability 495).
//!
//! Both extend `ChatChannelListV1`, domain 12 delta type 3, after its legacy fields 1..=3:
//! - Capability 494 `CHAT_JOINED_CHANNELS_V1` requires 80: repeated varint field 4
//!   `joined_channel`, the channels the character is in now (the own channel or an invitation of
//!   the same list), ascending and distinct, so its tabs survive a reconnect.
//! - Capability 495 `CHAT_CHANNEL_PARTICIPANTS_V1` requires 80: repeated message field 5
//!   `ChatChannelParticipantsV1 { uint32 channel_id = 1; repeated string name = 2; }`, one per
//!   channel the character is in, ascending and distinct, each with the 1..=17 members in it
//!   (owner and 16 invitations, `CHATPM-RL-01`) ordered by name. Its join, leave and exclude
//!   notices are line kind 8 (`line_extended`).
//!
//! A channel's label is the client's: "<owner>'s channel", from `owner_name` of its invitation
//! or the session's own name for its own channel. Without selection the list bytes are
//! unchanged, and the legacy [`decode_channel_list`] still refuses fields 4 and 5.
use super::private_channel::{
    ChatChannelList, MAX_CHAT_CHANNEL_LIST_BYTES, MAX_INVITED_PRIVATE_CHANNELS,
    MAX_PRIVATE_CHANNEL_INVITES, decode_channel_list, encode_channel_list,
};
use super::*;

/// Registered capability `CHAT_JOINED_CHANNELS_V1` (requires 80).
pub const CAPABILITY_CHAT_JOINED_CHANNELS_V1: u32 = 494;
/// Registered capability `CHAT_CHANNEL_PARTICIPANTS_V1` (requires 80).
pub const CAPABILITY_CHAT_CHANNEL_PARTICIPANTS_V1: u32 = 495;
/// The own channel and 16 invitations (`CHATPM-RL-02`).
pub const MAX_JOINED_PRIVATE_CHANNELS: usize = 1 + MAX_INVITED_PRIVATE_CHANNELS;
/// The owner and 16 invitations (`CHATPM-RL-01`).
pub const MAX_PRIVATE_CHANNEL_PARTICIPANTS: usize = 1 + MAX_PRIVATE_CHANNEL_INVITES;
/// The legacy 2,055 bytes, 17 joined channels of 1 + 3 and 17 participant entries of
/// 1 + 2 + (1 + 3) + 17 names of 1 + 1 + 120.
pub const MAX_CHAT_CHANNEL_LIST_EXTENDED_BYTES: usize = MAX_CHAT_CHANNEL_LIST_BYTES
    + MAX_JOINED_PRIVATE_CHANNELS * 4
    + MAX_JOINED_PRIVATE_CHANNELS * (3 + 4 + MAX_PRIVATE_CHANNEL_PARTICIPANTS * 122);
const _: () = assert!(MAX_CHAT_CHANNEL_LIST_EXTENDED_BYTES == 37_500);
const _: () = assert!(MAX_CHAT_CHANNEL_LIST_EXTENDED_BYTES <= crate::MAX_STATE_DELTA_PAYLOAD_BYTES);

/// The members of one channel the session is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelParticipants {
    pub channel_id: u16,
    pub names: Vec<String>,
}

/// Fields 4 and 5 of `ChatChannelListV1`; empty when neither capability is selected.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChannelListExtension {
    pub joined: Vec<u16>,
    pub participants: Vec<ChannelParticipants>,
}

/// Every referenced channel is listed; both lists ascend without repeats and stay in bounds.
fn check_extension(list: &ChatChannelList, extension: &ChannelListExtension) -> WireResult<()> {
    if extension.joined.len() > MAX_JOINED_PRIVATE_CHANNELS
        || extension.participants.len() > MAX_JOINED_PRIVATE_CHANNELS
    {
        return Err(ChatWireError::LimitExceeded);
    }
    let listed = |channel_id: u16| {
        list.own_channel == Some(channel_id)
            || list
                .invited
                .iter()
                .any(|invited| invited.channel_id == channel_id)
    };
    let ascending = |ids: &mut dyn Iterator<Item = u16>| {
        let mut last = 0;
        ids.all(|id| {
            let ok = id > last && listed(id);
            last = id;
            ok
        })
    };
    if !ascending(&mut extension.joined.iter().copied())
        || !ascending(&mut extension.participants.iter().map(|entry| entry.channel_id))
    {
        return Err(ChatWireError::Malformed);
    }
    for entry in &extension.participants {
        if entry.names.is_empty() {
            return Err(ChatWireError::Malformed);
        }
        if entry.names.len() > MAX_PRIVATE_CHANNEL_PARTICIPANTS {
            return Err(ChatWireError::LimitExceeded);
        }
        for name in &entry.names {
            check_text(name, MAX_CHAT_NAME_BYTES)?;
        }
    }
    Ok(())
}

/// Encodes a channel list delta (type 3) with the extension after the legacy fields. An empty
/// extension gives the legacy bytes. A list outside the bounds is a server fault.
pub fn encode_channel_list_extended(
    list: &ChatChannelList,
    extension: &ChannelListExtension,
) -> WireResult<Vec<u8>> {
    check_extension(list, extension)?;
    let mut out = encode_channel_list(list)?;
    for channel_id in &extension.joined {
        push_varint_field(&mut out, 4, u64::from(*channel_id));
    }
    for entry in &extension.participants {
        let mut inner = Vec::with_capacity(4 + entry.names.len() * (MAX_CHAT_NAME_BYTES + 2));
        push_varint_field(&mut inner, 1, u64::from(entry.channel_id));
        for name in &entry.names {
            push_message_field(&mut inner, 2, name.as_bytes());
        }
        push_message_field(&mut out, 5, &inner);
    }
    Ok(out)
}

/// Decodes a channel list delta of a session with capability 494 or 495; a legacy list decodes
/// with an empty extension.
pub fn decode_channel_list_extended(
    payload: &[u8],
) -> WireResult<(ChatChannelList, ChannelListExtension)> {
    if payload.len() > MAX_CHAT_CHANNEL_LIST_EXTENDED_BYTES {
        return Err(ChatWireError::LimitExceeded);
    }
    let mut extension = ChannelListExtension::default();
    let mut rest = Vec::with_capacity(payload.len().min(MAX_CHAT_CHANNEL_LIST_BYTES));
    let mut cursor = 0;
    while cursor < payload.len() {
        let start = cursor;
        let key = read_varint(payload, &mut cursor)?;
        match (key >> 3, key & 0x07) {
            (4, 0) => {
                if extension.joined.len() == MAX_JOINED_PRIVATE_CHANNELS {
                    return Err(ChatWireError::LimitExceeded);
                }
                extension.joined.push(read_channel(payload, &mut cursor)?);
            }
            (5, 2) => {
                if extension.participants.len() == MAX_JOINED_PRIVATE_CHANNELS {
                    return Err(ChatWireError::LimitExceeded);
                }
                let entry = read_participants(read_bytes(payload, &mut cursor)?)?;
                extension.participants.push(entry);
            }
            (_, wire) => {
                match wire {
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
    let list = decode_channel_list(&rest)?;
    check_extension(&list, &extension)?;
    Ok((list, extension))
}

fn read_channel(input: &[u8], cursor: &mut usize) -> WireResult<u16> {
    u16::try_from(read_uint32(input, cursor)?)
        .ok()
        .filter(|channel_id| *channel_id != 0)
        .ok_or(ChatWireError::Malformed)
}

fn read_participants(input: &[u8]) -> WireResult<ChannelParticipants> {
    let mut channel = None;
    let mut names = Vec::new();
    read_fields(input, |field, wire, input, cursor| match (field, wire) {
        (1, 0) => set_once(&mut channel, read_channel(input, cursor)?),
        (2, 2) => {
            if names.len() == MAX_PRIVATE_CHANNEL_PARTICIPANTS {
                return Err(ChatWireError::LimitExceeded);
            }
            names.push(read_text(input, cursor, MAX_CHAT_NAME_BYTES)?);
            Ok(())
        }
        _ => Err(ChatWireError::Malformed),
    })?;
    Ok(ChannelParticipants {
        channel_id: channel.ok_or(ChatWireError::Malformed)?,
        names,
    })
}

#[cfg(test)]
#[path = "private_channel_extended_tests.rs"]
mod tests;
