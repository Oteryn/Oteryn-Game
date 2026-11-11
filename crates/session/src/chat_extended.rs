//! WU26 chat extensions the client consumes: the private echo (493), the joined channels (494),
//! the channel participants and notices (495) and the line sequence (496). Each field is decoded
//! only when its capability is selected; without any of them the legacy decoders apply.
use super::{ChatChannelList, ChatLine, SessionError, guild_chat};
use oteryn_protocol_oteryn::chat::line_extended::{
    CAPABILITY_CHAT_LINE_SEQUENCE_V1, CAPABILITY_CHAT_PRIVATE_ECHO_V1, ChatLineBody,
    decode_chat_line_extended,
};
pub use oteryn_protocol_oteryn::chat::line_extended::{ChannelNotice, ChannelNoticeKind};
use oteryn_protocol_oteryn::chat::private_channel as wire;
use oteryn_protocol_oteryn::chat::private_channel_extended::{
    CAPABILITY_CHAT_CHANNEL_PARTICIPANTS_V1, CAPABILITY_CHAT_JOINED_CHANNELS_V1,
    decode_channel_list_extended,
};
pub use oteryn_protocol_oteryn::chat::private_channel_extended::{
    ChannelListExtension, ChannelParticipants,
};

/// Each extension capability and the capability it requires (CHAT_V1 or private channels).
pub(super) const REQUIRES: [(u32, u32); 4] = [
    (CAPABILITY_CHAT_PRIVATE_ECHO_V1, 7),
    (CAPABILITY_CHAT_JOINED_CHANNELS_V1, 80),
    (CAPABILITY_CHAT_CHANNEL_PARTICIPANTS_V1, 80),
    (CAPABILITY_CHAT_LINE_SEQUENCE_V1, 7),
];

/// Side data of one chat log line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineExtra {
    /// Capability 493: the line is the sender's own echo of a private message to this name.
    pub recipient_name: Option<String>,
    /// Capability 496: the line's sequence, strictly increasing within the session.
    pub line_sequence: Option<u64>,
}

/// A capability-495 notice with the order it arrived in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticeEntry {
    /// 1 for the first notice of the session, then increasing.
    pub serial: u64,
    pub notice: ChannelNotice,
}

pub(super) enum Decoded {
    Line { line: ChatLine, extra: LineExtra },
    Notice(ChannelNotice),
}

fn require(selected: &[u32], capability: u32) -> Result<(), SessionError> {
    if selected.contains(&capability) {
        Ok(())
    } else {
        Err(SessionError::CapabilityNotSelected { capability })
    }
}

/// Decodes a line delta. A session with none of 493, 495 and 496 uses the legacy decoder; one
/// with any of them refuses a field of a capability it did not select.
pub(super) fn decode_line(payload: &[u8], selected: &[u32]) -> Result<Decoded, SessionError> {
    if ![
        CAPABILITY_CHAT_PRIVATE_ECHO_V1,
        CAPABILITY_CHAT_CHANNEL_PARTICIPANTS_V1,
        CAPABILITY_CHAT_LINE_SEQUENCE_V1,
    ]
    .iter()
    .any(|capability| selected.contains(capability))
    {
        return Ok(Decoded::Line {
            line: guild_chat::decode_line(payload, selected)?,
            extra: LineExtra::default(),
        });
    }
    let decoded = decode_chat_line_extended(payload)?;
    if decoded.line_sequence.is_some() {
        require(selected, CAPABILITY_CHAT_LINE_SEQUENCE_V1)?;
    }
    let mut extra = LineExtra {
        recipient_name: None,
        line_sequence: decoded.line_sequence,
    };
    let line = match decoded.body {
        ChatLineBody::Line(line) => line,
        ChatLineBody::PrivateEcho(echo) => {
            require(selected, CAPABILITY_CHAT_PRIVATE_ECHO_V1)?;
            extra.recipient_name = Some(echo.recipient_name);
            ChatLine::Private {
                speaker_name: echo.speaker_name,
                text: echo.text,
            }
        }
        ChatLineBody::ChannelNotice(notice) => {
            require(selected, CAPABILITY_CHAT_CHANNEL_PARTICIPANTS_V1)?;
            return Ok(Decoded::Notice(notice));
        }
    };
    Ok(Decoded::Line {
        line: guild_chat::check_line(line, selected)?,
        extra,
    })
}

/// Decodes a channel list delta; fields 4 and 5 are taken only for the capability selected.
pub(super) fn decode_list(
    payload: &[u8],
    selected: &[u32],
) -> Result<(ChatChannelList, Option<ChannelListExtension>), SessionError> {
    let joined = selected.contains(&CAPABILITY_CHAT_JOINED_CHANNELS_V1);
    let participants = selected.contains(&CAPABILITY_CHAT_CHANNEL_PARTICIPANTS_V1);
    if !joined && !participants {
        return Ok((super::private_channels::decode(payload, selected)?, None));
    }
    require(selected, wire::CAPABILITY_CHAT_PRIVATE_CHANNEL_V1)?;
    let (list, extension) = decode_channel_list_extended(payload)?;
    if (!joined && !extension.joined.is_empty())
        || (!participants && !extension.participants.is_empty())
    {
        return Err(SessionError::SessionUnusable);
    }
    guild_chat::check_rooms(list.public_rooms, selected)?;
    Ok((list, Some(extension)))
}

impl super::ChatLog {
    /// Capabilities 493 and 496: the side data of each kept line, oldest first, in step with
    /// [`ChatLog::lines`](super::ChatLog::lines).
    pub fn line_extras(&self) -> impl ExactSizeIterator<Item = &LineExtra> {
        self.extras.iter()
    }

    /// Capability 495 notices, oldest first.
    pub fn channel_notices(&self) -> impl ExactSizeIterator<Item = &NoticeEntry> {
        self.notices.iter()
    }

    /// Capabilities 494 and 495: the joined channels and participants of the latest list.
    #[must_use]
    pub fn channel_extension(&self) -> Option<&ChannelListExtension> {
        self.channel_extension.as_ref()
    }

    pub(super) fn push_notice(&mut self, notice: ChannelNotice) {
        if self.notices.len() >= super::MAX_CHAT_LOG_LINES {
            self.notices.pop_front();
        }
        self.notice_serial += 1;
        self.notices.push_back(NoticeEntry {
            serial: self.notice_serial,
            notice,
        });
    }
}

#[cfg(test)]
#[path = "chat_extended_tests.rs"]
mod tests;
