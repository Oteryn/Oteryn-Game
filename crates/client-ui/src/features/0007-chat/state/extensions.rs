//! The WU26 chat extensions (capabilities 493-496) applied to the console. Every field of
//! [`ChatExtras`] is `None` while its capability is not selected, and the console then behaves
//! as before.
use super::{ChannelTab, ChatPanel, Console};
use crate::chat_channel_participants::{MAX_NOTICES, NoticeLine};
use oteryn_client_runtime::chat::ChatLine;
use oteryn_client_runtime::chat_private_channel::ChatChannelList;

/// What the session adds to the chat log, as the capabilities selected give it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatExtras {
    /// Capability 493: the recipient of each own echo, in step with the lines.
    pub recipients: Option<Vec<Option<String>>>,
    /// Capability 496: the sequence of each line, in step with the lines.
    pub sequences: Option<Vec<Option<u64>>>,
    /// Capability 494: the private channels the character is in.
    pub joined: Option<Vec<u16>>,
    /// Capability 495: the participants of each channel the character is in.
    pub participants: Option<Vec<(u16, Vec<String>)>>,
    /// Capability 495: the notices of the session.
    pub notices: Option<Vec<NoticeLine>>,
}

impl Console {
    /// Capability 494: open a tab for each private channel the character joined, once; the
    /// Join result opened its own, and a tab the player closed stays closed.
    pub fn sync_joined(&mut self, joined: &[u16]) {
        for id in crate::chat_joined_channels::newly_joined(&mut self.joined_seen, joined) {
            let tab = ChannelTab::Private(id);
            if !self.channels.contains(&tab) {
                self.channels.push(tab);
            }
        }
    }

    /// Capability 495: the owner of each channel (the own channel is the character's) and its
    /// participants.
    pub fn sync_participants(
        &mut self,
        own_name: &str,
        list: Option<&ChatChannelList>,
        participants: &[(u16, Vec<String>)],
    ) {
        self.owners.clear();
        if let Some(list) = list {
            if let Some(id) = list.own_channel.filter(|_| !own_name.is_empty()) {
                self.owners.insert(id, own_name.to_owned());
            }
            for invite in &list.invited {
                self.owners
                    .insert(invite.channel_id, invite.owner_name.clone());
            }
        }
        self.participants = participants.iter().cloned().collect();
    }

    /// Capability 495: keep the notices not taken yet.
    pub fn sync_notices(&mut self, notices: &[NoticeLine]) {
        let seen = self.notice_seen;
        for notice in notices.iter().filter(|notice| notice.serial > seen) {
            self.notice_seen = notice.serial;
            self.channel_notices.push(notice.clone());
        }
        let surplus = self.channel_notices.len().saturating_sub(MAX_NOTICES);
        self.channel_notices.drain(..surplus);
    }

    /// Capability 496: open a Conversation tab for each sender of a new private message.
    pub fn sync_conversations(
        &mut self,
        lines: &[ChatLine],
        sequences: &[Option<u64>],
        recipients: &[Option<String>],
    ) {
        let senders = crate::chat_line_sequence::new_senders(
            lines,
            sequences,
            recipients,
            &mut self.sequence_seen,
        );
        for name in senders {
            let open = self.channels.iter().any(
                |tab| matches!(tab, ChannelTab::Conversation(open) if open.eq_ignore_ascii_case(&name)),
            );
            if !open && self.communication.allows(&name) {
                self.channels.push(ChannelTab::Conversation(name));
            }
        }
    }

    /// A new session starts its own sequences and notices.
    pub fn reset_extensions(&mut self) {
        self.owners.clear();
        self.participants.clear();
        self.channel_notices.clear();
        self.joined_seen.clear();
        self.notice_seen = 0;
        self.sequence_seen = 0;
    }
}

impl ChatPanel {
    /// Takes the extensions of the capabilities selected; `lines` is already the log the
    /// extras belong to.
    pub fn apply_extras(
        &mut self,
        own_name: &str,
        list: Option<&ChatChannelList>,
        extras: ChatExtras,
    ) {
        self.recipients = extras.recipients.unwrap_or_default();
        if let Some(joined) = &extras.joined {
            self.console.sync_joined(joined);
        }
        if let Some(participants) = &extras.participants {
            self.console.sync_participants(own_name, list, participants);
        }
        if let Some(notices) = &extras.notices {
            self.console.sync_notices(notices);
        }
        if let Some(sequences) = &extras.sequences {
            self.console
                .sync_conversations(&self.lines, sequences, &self.recipients);
        }
    }
}
