//! Persisted console projections and geometry for the four existing spell-slot callers.
use super::SettingsState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConsolePreferences {
    pub visible: [bool; 5],
    pub mode_labels: bool,
    pub dropped: bool,
    pub feedback: bool,
    pub line_limit: u16,
}
impl Default for ConsolePreferences {
    fn default() -> Self {
        Self {
            visible: [true; 5],
            mode_labels: true,
            dropped: true,
            feedback: true,
            line_limit: 1000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ActionBarPreferences {
    pub visible: bool,
    pub origin: [u16; 2],
    pub columns: u8,
    pub size: [u16; 2],
    pub gap: u16,
}
impl Default for ActionBarPreferences {
    fn default() -> Self {
        Self {
            visible: true,
            origin: [8, 120],
            columns: 4,
            size: [64, 30],
            gap: 6,
        }
    }
}
impl ActionBarPreferences {
    pub(super) fn valid(&self) -> bool {
        (1..=4).contains(&self.columns)
            && self.origin.iter().all(|v| *v <= 4096)
            && self.size.iter().all(|v| (24..=160).contains(v))
            && self.gap <= 32
    }
    pub fn rects(&self, viewport: [u32; 2]) -> [[i32; 4]; 4] {
        let columns = u32::from(self.columns).clamp(1, 4);
        let [width, height] = self.size.map(u32::from);
        let gap = u32::from(self.gap);
        let x = u32::from(self.origin[0])
            .min(viewport[0].saturating_sub(columns * (width + gap) - gap));
        let y = u32::from(self.origin[1])
            .min(viewport[1].saturating_sub(4_u32.div_ceil(columns) * (height + gap) - gap));
        std::array::from_fn(|slot| {
            [
                (x + slot as u32 % columns * (width + gap)) as i32,
                (y + slot as u32 / columns * (height + gap)) as i32,
                width as i32,
                height as i32,
            ]
        })
    }
}
impl SettingsState {
    pub fn sync_presentation(&self, link: &crate::play::PlayLink) {
        let settings = &self.preferences.console;
        let presentation = oteryn_client_ui::chat::Presentation {
            visible: settings.visible,
            mode_labels: settings.mode_labels,
            dropped: settings.dropped,
            feedback: settings.feedback,
            limit: settings.line_limit,
        };
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.chat.console.presentation = presentation;
        mailbox.chat_public.console.presentation = presentation;
        mailbox.chat_private.presentation = presentation;
    }
    pub fn queue_bar_slot(&mut self, slot: usize) -> bool {
        let Some(id) = [
            "local-spell-action-bar-spell-1",
            "local-spell-action-bar-spell-2",
            "local-spell-action-bar-spell-3",
            "local-spell-action-bar-spell-4",
        ]
        .get(slot)
        .copied() else {
            return false;
        };
        if !self.preferences.action_bar.visible {
            return false;
        }
        if self.window_actions.len() >= 32 {
            return false;
        }
        self.window_actions.push_back(id);
        true
    }
}
