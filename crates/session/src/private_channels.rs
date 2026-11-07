//! Capability 80's existing channel-list delta, using the shared CHAT revision and codec.
use super::{ChatChannelList, ChatLog, Session, SessionError, SessionStream, guild_chat};
use oteryn_protocol_oteryn::chat::private_channel as wire;

pub(super) fn decode(payload: &[u8], selected: &[u32]) -> Result<ChatChannelList, SessionError> {
    if !selected.contains(&wire::CAPABILITY_CHAT_PRIVATE_CHANNEL_V1) {
        return Err(SessionError::CapabilityNotSelected {
            capability: wire::CAPABILITY_CHAT_PRIVATE_CHANNEL_V1,
        });
    }
    let list = wire::decode_channel_list(payload)?;
    guild_chat::check_rooms(list.public_rooms, selected)?;
    Ok(list)
}

impl ChatLog {
    /// Available public rooms, the owned private channel and current invitations.
    /// `None` until the first capability-80 list arrives; an empty list replaces old state.
    /// This is server-supplied presentation state, not client authority to join a channel.
    #[must_use]
    pub fn private_channels(&self) -> Option<&ChatChannelList> {
        self.private_channels.as_ref()
    }
}

impl<S: SessionStream> Session<S> {
    /// The latest validated capability-80 list, distinct from the currently open room set.
    /// `None` without capability 80 or before the first list delta.
    #[must_use]
    pub fn private_channel_list(&self) -> Option<&ChatChannelList> {
        self.chat.as_ref()?.private_channels()
    }
}

#[cfg(test)]
#[path = "private_channels_tests.rs"]
mod tests;
