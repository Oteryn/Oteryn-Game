//! Character collection launchers; each browser owns its native handles.
use super::{gallery_window, items_window, protection_window};
use crate::{combat_input::PlayCommand, play::mailbox::Mailbox};
use oteryn_client_ui::cyclopedia_character::CyclopediaCharacterPanel;
use std::sync::{Mutex, Weak};
use tokio::sync::mpsc::UnboundedSender;
pub(super) enum Browser {
    Achievements,
    Appearances,
    Items,
    Protection,
    Sanctions,
}
impl Browser {
    pub(super) fn for_action(action: usize, p: &CyclopediaCharacterPanel) -> Option<Self> {
        match action {
            119 => Some(Self::Protection),
            140 => Some(Self::Achievements),
            141 if p.target.is_none() => Some(Self::Sanctions),
            105 if p.target.is_none() => Some(Self::Appearances),
            118 if p.target.is_none() => Some(Self::Items),
            _ => None,
        }
    }
    pub(super) fn open(self, weak: Weak<Mutex<Mailbox>>, commands: UnboundedSender<PlayCommand>) {
        match self {
            Self::Achievements => {
                crate::play::feature_local_account_achievements::view::open_character(
                    weak, commands,
                )
            }
            Self::Appearances => gallery_window::open(weak, commands),
            Self::Items => items_window::open(weak, commands),
            Self::Protection => protection_window::open(weak, commands),
            Self::Sanctions => {
                #[cfg(windows)]
                if let Some(mailbox) = weak.upgrade() {
                    let link = crate::play::PlayLink {
                        owns_session: false,
                        mailbox,
                        commands,
                    };
                    super::sanctions::open_window(&link);
                }
                #[cfg(not(windows))]
                let _ = (weak, commands);
            }
        }
    }
}
