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
}
impl UiSound {
    pub fn frequency(self) -> f32 {
        match self {
            Self::Interaction => 440.0,
            Self::Party => 660.0,
            Self::Vip => 880.0,
            Self::Chat => 550.0,
        }
    }
}
pub(in crate::play) fn observe(mailbox: &mut mailbox::Mailbox, events: &[SessionEvent]) {
    for event in events {
        let cue = match event {
            SessionEvent::ChatLine(delta) => match delta.value {
                ChatLine::Party { .. } => Some(UiSound::Party),
                ChatLine::Dropped => None,
                _ => Some(UiSound::Chat),
            },
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
        let filters = self.preferences().audio.ui_sounds;
        let enabled = !self.preferences().audio.muted;
        std::mem::take(&mut self.ui_sounds)
            .into_iter()
            .filter(|cue| enabled && filters[*cue as usize])
            .collect()
    }
}
