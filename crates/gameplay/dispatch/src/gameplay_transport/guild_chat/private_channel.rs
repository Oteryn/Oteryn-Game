//! Private channels of capability 80 `CHAT_PRIVATE_CHANNEL_V1` (CHAT-PM-CHANNELS-1).
//!
//! A channel lives in the chat state of one Channel, under the same lock as the rest of it: its
//! owner, invitations and members are admitted sessions of that Channel. It closes when its owner
//! leaves it or its session ends (Canary `Chat::removeUserFromChannel`), and nothing is stored.
//! Invitations are bounded per channel (`CHATPM-RL-01`) and per invitee (`CHATPM-RL-02`). A line
//! passes the CHAT_V1 limiter and text rule and reaches every joined member, the speaker
//! included (Canary `ChatChannel::talk`).
use super::*;
use oteryn_protocol_oteryn::chat::private_channel::{
    ChatChannelList, InvitedChannel, MAX_INVITED_PRIVATE_CHANNELS, MAX_PRIVATE_CHANNEL_INVITES,
    PrivateChannelIntent, PrivateChannelLine,
};

/// One session's private-channel state.
#[derive(Debug, Default)]
pub(super) struct PrivateChannels {
    /// The session selected capability 80; without it the session takes no part.
    enabled: bool,
    own: Option<OwnChannel>,
    /// Its channel list changed since its client was last sent one.
    dirty: bool,
}

#[derive(Debug)]
struct OwnChannel {
    id: u16,
    invited: Vec<GameSessionId>,
    members: Vec<GameSessionId>,
}

type Sessions = HashMap<GameSessionId, ChatSession>;

fn mark(sessions: &mut Sessions, session: GameSessionId) {
    if let Some(entry) = sessions.get_mut(&session) {
        entry.channels.dirty = true;
    }
}

/// The owner of channel `id`.
fn owner_of(sessions: &Sessions, id: u16) -> Option<GameSessionId> {
    sessions
        .iter()
        .find(|(_, entry)| entry.channels.own.as_ref().is_some_and(|own| own.id == id))
        .map(|(session, _)| *session)
}

/// The enabled session of this Channel playing `name`.
fn session_named(sessions: &Sessions, name: &CharacterName) -> Option<(GameSessionId, bool)> {
    let key = name.comparison_key();
    sessions.iter().find_map(|(session, entry)| {
        CharacterName::parse(&entry.name)
            .is_ok_and(|own| own.comparison_key() == key)
            .then_some((*session, entry.channels.enabled))
    })
}

fn invitations_of(sessions: &Sessions, session: GameSessionId) -> usize {
    sessions
        .values()
        .filter_map(|entry| entry.channels.own.as_ref())
        .filter(|own| own.invited.contains(&session))
        .count()
}

/// Closes the channel `session` owns: its invitees learn it from their next list.
fn close_own(sessions: &mut Sessions, session: GameSessionId) {
    let Some(own) = sessions
        .get_mut(&session)
        .and_then(|entry| entry.channels.own.take())
    else {
        return;
    };
    mark(sessions, session);
    for invitee in own.invited {
        mark(sessions, invitee);
    }
}

/// Withdraws every invitation and membership `session` holds in other channels.
fn leave_all(sessions: &mut Sessions, session: GameSessionId) {
    for entry in sessions.values_mut() {
        if let Some(own) = entry.channels.own.as_mut() {
            own.invited.retain(|invitee| *invitee != session);
            own.members.retain(|member| *member != session);
        }
    }
}

impl ChatRuntime {
    /// Negotiation of `session` (admission, reconnect or transfer): with capability 80 it takes
    /// part and is sent its channel list; without it its channels close and it leaves the others.
    pub fn set_private_channels(&self, session: GameSessionId, selected: bool) {
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        if !selected {
            close_own(&mut sessions, session);
            leave_all(&mut sessions, session);
        }
        if let Some(entry) = sessions.get_mut(&session) {
            entry.channels.enabled = selected;
            entry.channels.dirty = selected;
        }
    }

    /// The session's own channel closes and it leaves the others (its session ended).
    pub(super) fn forget_private_channels(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.sessions.lock() {
            close_own(&mut sessions, session);
            leave_all(&mut sessions, session);
        }
    }

    /// The channel list of `session` when it changed since its client was last sent one (a
    /// domain 12 delta type 3); `None` when unchanged or without capability 80.
    pub fn take_channel_list(&self, session: GameSessionId) -> Option<ChatChannelList> {
        let mut sessions = self.sessions.lock().ok()?;
        let entry = sessions.get_mut(&session)?;
        if !(entry.channels.enabled && std::mem::take(&mut entry.channels.dirty)) {
            return None;
        }
        let own_channel = entry.channels.own.as_ref().map(|own| own.id);
        let mut public_rooms = ChatRoomSet::default();
        if self.relay().is_some() {
            for room in ChatRoom::ALL {
                public_rooms.insert(room);
            }
        }
        let mut invited: Vec<InvitedChannel> = sessions
            .values()
            .filter_map(|owner| {
                let own = owner.channels.own.as_ref()?;
                own.invited.contains(&session).then(|| InvitedChannel {
                    channel_id: own.id,
                    owner_name: owner.name.clone(),
                })
            })
            .collect();
        invited.sort_by_key(|channel| channel.channel_id);
        Some(ChatChannelList {
            public_rooms,
            own_channel,
            invited,
        })
    }

    /// Opens the own channel of `session`; `premium` is its current Premium (Canary
    /// `Chat::createChannel`: Premium only, one channel per player). The owner joins it.
    pub fn create_private_channel(
        &self,
        session: GameSessionId,
        premium: bool,
    ) -> ChatIntentResult {
        let Ok(mut sessions) = self.sessions.lock() else {
            return result(ChatDisposition::ChatUnavailable);
        };
        let Some(entry) = sessions.get(&session) else {
            return result(ChatDisposition::ChatUnavailable);
        };
        if !entry.channels.enabled {
            return result(ChatDisposition::Rejected);
        }
        if entry.channels.own.is_some() {
            return result(ChatDisposition::Rejected);
        }
        if !premium {
            return result(ChatDisposition::NotAllowed);
        }
        let Some(id) = (1..=u16::MAX).find(|id| owner_of(&sessions, *id).is_none()) else {
            return result(ChatDisposition::ChatUnavailable);
        };
        if let Some(entry) = sessions.get_mut(&session) {
            entry.channels.own = Some(OwnChannel {
                id,
                invited: Vec::new(),
                members: vec![session],
            });
            entry.channels.dirty = true;
        }
        result(ChatDisposition::Ok)
    }

    /// Every private-channel intent but `Create`, answered in full under the chat lock.
    pub(super) fn private_channel(
        &self,
        session: GameSessionId,
        intent: &PrivateChannelIntent,
        now: SemanticTimeMicros,
    ) -> ChatIntentResult {
        // Unusable text or name is refused before the lock.
        let name = match intent {
            PrivateChannelIntent::Say { text, .. } if ChatText::parse(text).is_err() => {
                return result(ChatDisposition::Rejected);
            }
            PrivateChannelIntent::Invite { name } | PrivateChannelIntent::Exclude { name } => {
                match CharacterName::parse(name) {
                    Ok(name) => Some(name),
                    Err(_) => return result(ChatDisposition::NotOnline),
                }
            }
            _ => None,
        };
        let Ok(mut sessions) = self.sessions.lock() else {
            return result(ChatDisposition::ChatUnavailable);
        };
        let Some(entry) = sessions.get(&session) else {
            return result(ChatDisposition::ChatUnavailable);
        };
        if !entry.channels.enabled {
            return result(ChatDisposition::Rejected);
        }
        match (intent, name) {
            (PrivateChannelIntent::Invite { .. }, Some(name)) => {
                invite(&mut sessions, session, &name)
            }
            (PrivateChannelIntent::Exclude { .. }, Some(name)) => {
                exclude(&mut sessions, session, &name)
            }
            (PrivateChannelIntent::Join { channel_id }, _) => {
                join(&mut sessions, session, *channel_id)
            }
            (PrivateChannelIntent::Leave { channel_id }, _) => {
                leave(&mut sessions, session, *channel_id)
            }
            (PrivateChannelIntent::Say { channel_id, text }, _) => {
                say(&mut sessions, session, *channel_id, text, now)
            }
            (PrivateChannelIntent::List, _) => {
                mark(&mut sessions, session);
                result(ChatDisposition::Ok)
            }
            _ => result(ChatDisposition::Rejected),
        }
    }
}

/// Canary `Game::playerChannelInvite`, `PrivateChatChannel::invitePlayer`.
fn invite(
    sessions: &mut Sessions,
    session: GameSessionId,
    name: &CharacterName,
) -> ChatIntentResult {
    if sessions
        .get(&session)
        .is_none_or(|entry| entry.channels.own.is_none())
    {
        return result(ChatDisposition::NotAllowed);
    }
    let Some((target, enabled)) = session_named(sessions, name) else {
        return result(ChatDisposition::NotOnline);
    };
    if target == session || !enabled {
        return result(ChatDisposition::Rejected);
    }
    let held = invitations_of(sessions, target);
    let Some(own) = sessions
        .get_mut(&session)
        .and_then(|entry| entry.channels.own.as_mut())
    else {
        return result(ChatDisposition::NotAllowed);
    };
    if own.invited.contains(&target) {
        return result(ChatDisposition::Ok);
    }
    if own.invited.len() >= MAX_PRIVATE_CHANNEL_INVITES || held >= MAX_INVITED_PRIVATE_CHANNELS {
        return result(ChatDisposition::Rejected);
    }
    own.invited.push(target);
    mark(sessions, target);
    result(ChatDisposition::Ok)
}

/// Canary `Game::playerChannelExclude`, `PrivateChatChannel::excludePlayer`: the invitation is
/// withdrawn and the character leaves the channel.
fn exclude(
    sessions: &mut Sessions,
    session: GameSessionId,
    name: &CharacterName,
) -> ChatIntentResult {
    if sessions
        .get(&session)
        .is_none_or(|entry| entry.channels.own.is_none())
    {
        return result(ChatDisposition::NotAllowed);
    }
    let Some((target, _)) = session_named(sessions, name) else {
        return result(ChatDisposition::NotOnline);
    };
    if target == session {
        return result(ChatDisposition::Rejected);
    }
    let Some(own) = sessions
        .get_mut(&session)
        .and_then(|entry| entry.channels.own.as_mut())
    else {
        return result(ChatDisposition::NotAllowed);
    };
    if !own.invited.contains(&target) {
        return result(ChatDisposition::NotInvited);
    }
    own.invited.retain(|invitee| *invitee != target);
    own.members.retain(|member| *member != target);
    mark(sessions, target);
    result(ChatDisposition::Ok)
}

/// The channel `id` when `session` owns it or is invited to it (Canary `Chat::getChannel`).
fn admitted_to(
    sessions: &mut Sessions,
    session: GameSessionId,
    id: u16,
) -> Option<&mut OwnChannel> {
    let owner = owner_of(sessions, id)?;
    let own = sessions.get_mut(&owner)?.channels.own.as_mut()?;
    (owner == session || own.invited.contains(&session)).then_some(own)
}

/// Canary `Game::playerOpenChannel`.
fn join(sessions: &mut Sessions, session: GameSessionId, id: u16) -> ChatIntentResult {
    let Some(own) = admitted_to(sessions, session, id) else {
        return result(ChatDisposition::NotInvited);
    };
    if !own.members.contains(&session) {
        own.members.push(session);
    }
    result(ChatDisposition::Ok)
}

/// Canary `Game::playerCloseChannel`, `Chat::removeUserFromChannel`: the owner leaving closes it.
fn leave(sessions: &mut Sessions, session: GameSessionId, id: u16) -> ChatIntentResult {
    if owner_of(sessions, id) == Some(session) {
        close_own(sessions, session);
    } else if let Some(own) = admitted_to(sessions, session, id) {
        own.members.retain(|member| *member != session);
    }
    result(ChatDisposition::Ok)
}

/// Canary `Game::playerTalkToChannel`, `ChatChannel::talk`: only a member speaks, and every
/// member hears, the speaker included.
fn say(
    sessions: &mut Sessions,
    session: GameSessionId,
    id: u16,
    text: &str,
    now: SemanticTimeMicros,
) -> ChatIntentResult {
    let Some(own) = admitted_to(sessions, session, id) else {
        return result(ChatDisposition::NotInvited);
    };
    if !own.members.contains(&session) {
        return result(ChatDisposition::RoomNotOpen);
    }
    let members = own.members.clone();
    let Ok(text) = ChatText::parse(text) else {
        return result(ChatDisposition::Rejected);
    };
    let Some(speaker) = sessions.get_mut(&session) else {
        return result(ChatDisposition::ChatUnavailable);
    };
    if let Err(refused) = speaker.limiter.pre_gate_relay(false, now) {
        return refusal(refused);
    }
    let line = ChatLine::PrivateChannel(PrivateChannelLine {
        channel_id: id,
        speaker_name: speaker.name.clone(),
        text: text.as_str().to_owned(),
    });
    for member in members {
        if let Some(entry) = sessions.get_mut(&member) {
            entry.egress.push(line.clone());
        }
    }
    result(ChatDisposition::Ok)
}

#[cfg(test)]
#[path = "private_channel_tests.rs"]
mod tests;
