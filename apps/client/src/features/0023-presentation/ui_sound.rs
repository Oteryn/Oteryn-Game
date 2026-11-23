//! Local notification tones derived from confirmed protocol events; no server cue IDs invented.
use crate::play::{PlayLink, PlayView, mailbox};
use oteryn_session::{ChatLine, SessionEvent};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum UiSound {
    Interaction,
    Party,
    Vip,
    Chat,
    PartyMessage,
    Guild,
    Global,
    Npc,
    Private,
    PrivateWithoutTab,
    System,
}
impl UiSound {
    pub fn frequency(self) -> f32 {
        match self {
            Self::Interaction => 440.0,
            Self::Party => 660.0,
            Self::Vip => 880.0,
            Self::PartyMessage => 660.0,
            Self::Guild
            | Self::Global
            | Self::Npc
            | Self::Private
            | Self::PrivateWithoutTab
            | Self::System
            | Self::Chat => 550.0,
        }
    }
}
pub(in crate::play) fn observe(mailbox: &mut mailbox::Mailbox, events: &[SessionEvent]) {
    for event in events {
        let cue = match event {
            SessionEvent::ChatLine(delta)
                if !mailbox.chat.console.communication.visible(&delta.value) =>
            {
                None
            }
            SessionEvent::ChatLine(delta) => match &delta.value {
                ChatLine::Party { .. } => Some(UiSound::PartyMessage),
                ChatLine::Guild(_) => Some(UiSound::Guild),
                ChatLine::Room {
                    room: oteryn_session::ChatRoom::World,
                    ..
                } => Some(UiSound::Global),
                ChatLine::Private { speaker_name, .. } => {
                    let open = mailbox.chat.console.channels.iter().any(|tab| matches!(tab,
                        oteryn_client_ui::chat::ChannelTab::Conversation(name) if name.eq_ignore_ascii_case(speaker_name)));
                    Some(if open {
                        UiSound::Private
                    } else {
                        UiSound::PrivateWithoutTab
                    })
                }
                ChatLine::Dropped => None,
                _ => Some(UiSound::Chat),
            },
            SessionEvent::ServerMessage(_) => Some(UiSound::System),
            SessionEvent::NpcConversation(delta) if !delta.value.lines.is_empty() => {
                Some(UiSound::Npc)
            }
            SessionEvent::OpenContainer(_) | SessionEvent::NpcConversation(_) => {
                Some(UiSound::Interaction)
            }
            SessionEvent::VipPresence(delta) => {
                let mut announce = false;
                for entry in &delta.value.entries {
                    let old = mailbox.ui_vip_status.insert(entry.entry, entry.status);
                    announce |= entry.announce
                        && old != Some(entry.status)
                        && entry.status != oteryn_session::VipPresenceStatus::Unconfirmed;
                }
                mailbox
                    .ui_vip_status
                    .retain(|id, _| delta.value.entries.iter().any(|entry| entry.entry == *id));
                announce.then_some(UiSound::Vip)
            }
            _ => None,
        };
        if let Some(cue) = cue
            && mailbox.ui_sounds.len() < 32
        {
            mailbox.ui_sounds.push(cue);
        }
    }
}
impl PlayView {
    pub(in crate::play) fn poll_ui_sounds(&mut self, link: &PlayLink) {
        let incoming = std::mem::take(&mut mailbox::lock(&link.mailbox).ui_sounds);
        let available = 32usize.saturating_sub(self.ui_sounds.len());
        self.ui_sounds.extend(incoming.into_iter().take(available));
    }
    pub fn take_ui_sounds(&mut self) -> Vec<UiSound> {
        let audio = self.preferences().audio;
        std::mem::take(&mut self.ui_sounds)
            .into_iter()
            .filter_map(|cue| {
                let (tone, enabled) = match cue {
                    UiSound::PartyMessage => {
                        (UiSound::Party, audio.party_messages && audio.ui_sounds[3])
                    }
                    UiSound::Guild => (UiSound::Chat, audio.guild && audio.ui_sounds[3]),
                    UiSound::Global => (UiSound::Chat, audio.global && audio.ui_sounds[3]),
                    UiSound::Npc => (UiSound::Chat, audio.npc && audio.ui_sounds[3]),
                    UiSound::Private => (UiSound::Chat, audio.private && audio.ui_sounds[3]),
                    UiSound::PrivateWithoutTab => (
                        UiSound::Chat,
                        audio.private_without_tab && audio.ui_sounds[3],
                    ),
                    UiSound::System => (UiSound::Chat, audio.system && audio.ui_sounds[3]),
                    UiSound::Interaction | UiSound::Party | UiSound::Vip | UiSound::Chat => {
                        (cue, audio.ui_sounds[cue as usize])
                    }
                };
                (enabled && !audio.muted).then_some(tone)
            })
            .collect()
    }
}
