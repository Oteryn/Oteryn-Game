//! Negotiated CHAT_V1 private-channel extension (CHAT-PM-CHANNELS-1).
//!
//! Capability 80 `CHAT_PRIVATE_CHANNEL_V1` requires 7 and adds intent tags 7..=13, disposition 11
//! `NOT_INVITED`, line kind 6 (fields 4, 7, 11) and domain 12 delta type 3, the channel list.
//! Without selection every CHAT_V1 byte and behavior is unchanged. A private channel is named by
//! its Channel-local number, never by its owner's CharacterId.
use super::*;

pub const CAPABILITY_CHAT_PRIVATE_CHANNEL_V1: u32 = 80;
/// Delta type 3: the channel list, replaced whole.
pub const DELTA_TYPE_CHAT_CHANNEL_LIST_V1: u32 = 3;
/// `CHATPM-RL-01`: the characters one private channel can invite.
pub const MAX_PRIVATE_CHANNEL_INVITES: usize = 16;
/// `CHATPM-RL-02`: the private channels one character can be invited to.
pub const MAX_INVITED_PRIVATE_CHANNELS: usize = 16;
/// Public rooms 1 + 1, own channel 1 + 3, 16 invitations of 1 + 1 + (1 + 3) + (1 + 1 + 120).
pub const MAX_CHAT_CHANNEL_LIST_BYTES: usize = 2_054;

const LINE_KIND_PRIVATE_CHANNEL: u32 = 6;

/// A private-channel intent, oneof tags 7..=13 of `ChatIntentV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivateChannelIntent {
    /// Open the speaker's own channel.
    Create,
    /// Invite an online character to the speaker's own channel.
    Invite { name: String },
    /// Withdraw an invitation and remove the character from the speaker's own channel.
    Exclude { name: String },
    /// Join a channel the speaker owns or is invited to.
    Join { channel_id: u16 },
    /// Leave a channel; the owner leaving closes it.
    Leave { channel_id: u16 },
    /// One line to a channel the speaker has joined.
    Say { channel_id: u16, text: String },
    /// Send the channel list (delta type 3) again.
    List,
}

/// Line kind 6: one line of a private channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateChannelLine {
    pub channel_id: u16,
    pub speaker_name: String,
    pub text: String,
}

/// An invitation the session holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitedChannel {
    pub channel_id: u16,
    pub owner_name: String,
}

/// `ChatChannelListV1`: the public rooms that can be opened, the session's own channel and the
/// channels it is invited to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatChannelList {
    pub public_rooms: ChatRoomSet,
    pub own_channel: Option<u16>,
    pub invited: Vec<InvitedChannel>,
}

fn check_channel(channel_id: u16) -> WireResult<()> {
    if channel_id == 0 {
        return Err(ChatWireError::Malformed);
    }
    Ok(())
}

fn read_channel(input: &[u8], cursor: &mut usize) -> WireResult<u16> {
    let channel_id =
        u16::try_from(read_uint32(input, cursor)?).map_err(|_| ChatWireError::Malformed)?;
    check_channel(channel_id)?;
    Ok(channel_id)
}

pub(super) fn encode_intent(intent: &PrivateChannelIntent) -> WireResult<Vec<u8>> {
    let mut inner = Vec::new();
    let field = match intent {
        PrivateChannelIntent::Create => 7,
        PrivateChannelIntent::Invite { name } | PrivateChannelIntent::Exclude { name } => {
            check_text(name, MAX_CHAT_NAME_BYTES)?;
            push_message_field(&mut inner, 1, name.as_bytes());
            if matches!(intent, PrivateChannelIntent::Invite { .. }) {
                8
            } else {
                9
            }
        }
        PrivateChannelIntent::Join { channel_id } | PrivateChannelIntent::Leave { channel_id } => {
            check_channel(*channel_id)?;
            push_varint_field(&mut inner, 1, u64::from(*channel_id));
            if matches!(intent, PrivateChannelIntent::Join { .. }) {
                10
            } else {
                11
            }
        }
        PrivateChannelIntent::Say { channel_id, text } => {
            check_channel(*channel_id)?;
            check_text(text, MAX_CHAT_TEXT_BYTES)?;
            push_varint_field(&mut inner, 1, u64::from(*channel_id));
            push_message_field(&mut inner, 2, text.as_bytes());
            12
        }
        PrivateChannelIntent::List => 13,
    };
    let mut output = Vec::with_capacity(inner.len() + 3);
    push_message_field(&mut output, field, &inner);
    Ok(output)
}

pub(super) fn decode_intent(field: u64, input: &[u8]) -> WireResult<ChatIntent> {
    let (mut name, mut channel, mut text) = (None, None, None);
    read_fields(input, |number, wire, input, cursor| {
        match (field, number, wire) {
            (8 | 9, 1, 2) => set_once(&mut name, read_text(input, cursor, MAX_CHAT_NAME_BYTES)?),
            (10..=12, 1, 0) => set_once(&mut channel, read_channel(input, cursor)?),
            (12, 2, 2) => set_once(&mut text, read_text(input, cursor, MAX_CHAT_TEXT_BYTES)?),
            _ => Err(ChatWireError::Malformed),
        }
    })?;
    let malformed = || ChatWireError::Malformed;
    let name = || name.clone().ok_or_else(malformed);
    let channel_id = || channel.ok_or_else(malformed);
    Ok(ChatIntent::PrivateChannel(match field {
        7 => PrivateChannelIntent::Create,
        8 => PrivateChannelIntent::Invite { name: name()? },
        9 => PrivateChannelIntent::Exclude { name: name()? },
        10 => PrivateChannelIntent::Join {
            channel_id: channel_id()?,
        },
        11 => PrivateChannelIntent::Leave {
            channel_id: channel_id()?,
        },
        12 => PrivateChannelIntent::Say {
            channel_id: channel_id()?,
            text: text.ok_or_else(malformed)?,
        },
        13 => PrivateChannelIntent::List,
        _ => return Err(malformed()),
    }))
}

pub(super) fn encode_line(line: &PrivateChannelLine) -> WireResult<Vec<u8>> {
    check_channel(line.channel_id)?;
    check_text(&line.speaker_name, MAX_CHAT_NAME_BYTES)?;
    check_text(&line.text, MAX_CHAT_TEXT_BYTES)?;
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(LINE_KIND_PRIVATE_CHANNEL));
    push_message_field(&mut out, 4, line.speaker_name.as_bytes());
    push_message_field(&mut out, 7, line.text.as_bytes());
    push_varint_field(&mut out, 11, u64::from(line.channel_id));
    Ok(out)
}

pub(super) fn decode_line_if_private_channel(
    input: &[u8],
) -> WireResult<Option<PrivateChannelLine>> {
    // Inspect only the kind, then reject every alien or repeated field.
    let mut kind = None;
    read_fields(input, |field, wire, input, cursor| {
        if (field, wire) == (1, 0) {
            return set_once(&mut kind, read_uint32(input, cursor)?);
        }
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
    })?;
    if kind != Some(LINE_KIND_PRIVATE_CHANNEL) {
        return Ok(None);
    }
    let (mut channel, mut name, mut text) = (None, None, None);
    read_fields(input, |field, wire, input, cursor| match (field, wire) {
        (1, 0) => {
            read_uint32(input, cursor)?;
            Ok(())
        }
        (4, 2) => set_once(&mut name, read_text(input, cursor, MAX_CHAT_NAME_BYTES)?),
        (7, 2) => set_once(&mut text, read_text(input, cursor, MAX_CHAT_TEXT_BYTES)?),
        (11, 0) => set_once(&mut channel, read_channel(input, cursor)?),
        _ => Err(ChatWireError::Malformed),
    })?;
    Ok(Some(PrivateChannelLine {
        channel_id: channel.ok_or(ChatWireError::Malformed)?,
        speaker_name: name.ok_or(ChatWireError::Malformed)?,
        text: text.ok_or(ChatWireError::Malformed)?,
    }))
}

fn check_list(list: &ChatChannelList) -> WireResult<()> {
    if list.invited.len() > MAX_INVITED_PRIVATE_CHANNELS {
        return Err(ChatWireError::LimitExceeded);
    }
    if let Some(own) = list.own_channel {
        check_channel(own)?;
    }
    for (index, channel) in list.invited.iter().enumerate() {
        check_channel(channel.channel_id)?;
        check_text(&channel.owner_name, MAX_CHAT_NAME_BYTES)?;
        // A channel is listed once, and the own channel is never an invitation.
        if list.own_channel == Some(channel.channel_id)
            || list.invited[..index]
                .iter()
                .any(|earlier| earlier.channel_id == channel.channel_id)
        {
            return Err(ChatWireError::Malformed);
        }
    }
    Ok(())
}

/// Encodes a channel list delta (type 3). A list outside the bounds is a server fault.
pub fn encode_channel_list(list: &ChatChannelList) -> WireResult<Vec<u8>> {
    check_list(list)?;
    let mut out = Vec::with_capacity(MAX_CHAT_CHANNEL_LIST_BYTES);
    push_nonzero_varint_field(&mut out, 1, u64::from(list.public_rooms.0));
    if let Some(own) = list.own_channel {
        push_varint_field(&mut out, 2, u64::from(own));
    }
    for channel in &list.invited {
        let mut inner = Vec::with_capacity(4 + 2 + channel.owner_name.len());
        push_varint_field(&mut inner, 1, u64::from(channel.channel_id));
        push_message_field(&mut inner, 2, channel.owner_name.as_bytes());
        push_message_field(&mut out, 3, &inner);
    }
    Ok(out)
}

pub fn decode_channel_list(payload: &[u8]) -> WireResult<ChatChannelList> {
    if payload.len() > MAX_CHAT_CHANNEL_LIST_BYTES {
        return Err(ChatWireError::LimitExceeded);
    }
    let (mut rooms, mut own) = (None, None);
    let mut invited = Vec::new();
    read_fields(payload, |field, wire, input, cursor| match (field, wire) {
        (1, 0) => {
            let bits = u8::try_from(read_uint32(input, cursor)?)
                .ok()
                .filter(|bits| *bits != 0 && bits & !ChatRoomSet::ALL_BITS == 0)
                .ok_or(ChatWireError::Malformed)?;
            set_once(&mut rooms, ChatRoomSet(bits))
        }
        (2, 0) => set_once(&mut own, read_channel(input, cursor)?),
        (3, 2) => {
            if invited.len() == MAX_INVITED_PRIVATE_CHANNELS {
                return Err(ChatWireError::LimitExceeded);
            }
            let (mut channel, mut owner) = (None, None);
            read_fields(
                read_bytes(input, cursor)?,
                |field, wire, input, cursor| match (field, wire) {
                    (1, 0) => set_once(&mut channel, read_channel(input, cursor)?),
                    (2, 2) => set_once(&mut owner, read_text(input, cursor, MAX_CHAT_NAME_BYTES)?),
                    _ => Err(ChatWireError::Malformed),
                },
            )?;
            invited.push(InvitedChannel {
                channel_id: channel.ok_or(ChatWireError::Malformed)?,
                owner_name: owner.ok_or(ChatWireError::Malformed)?,
            });
            Ok(())
        }
        _ => Err(ChatWireError::Malformed),
    })?;
    let list = ChatChannelList {
        public_rooms: rooms.unwrap_or_default(),
        own_channel: own,
        invited,
    };
    check_list(&list)?;
    Ok(list)
}

#[cfg(test)]
#[path = "private_channel_tests.rs"]
mod tests;
