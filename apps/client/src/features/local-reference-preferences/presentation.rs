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
