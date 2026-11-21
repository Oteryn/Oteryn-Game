//! Default movement presets follow the existing console's on/off/temporary composer.
use super::*;
use crate::play::feature_local_reference_preferences::settings::ChatMode;
use oteryn_client_ui::chat::ComposerMode;

/// The mode the player chose: a one-message composer belongs to Chat Off.
fn chosen(mode: ComposerMode) -> ComposerMode {
    if mode == ComposerMode::Temporary {
        ComposerMode::Off
    } else {
        mode
    }
}
fn persisted(mode: ComposerMode) -> ChatMode {
    match mode {
        ComposerMode::On => ChatMode::On,
        ComposerMode::Auto => ChatMode::Auto,
        ComposerMode::Off | ComposerMode::Temporary => ChatMode::Off,
    }
}
fn composer(mode: ChatMode) -> ComposerMode {
    match mode {
        ChatMode::On => ComposerMode::On,
        ChatMode::Auto => ComposerMode::Auto,
        ChatMode::Off => ComposerMode::Off,
    }
}

impl SettingsState {
    pub fn apply_chat_preset(&mut self, chat_on: bool) -> Result<(), PreferenceError> {
        let mut preferences = self.preferences.clone();
        preferences.general_hotkey_preset(chat_on)?;
        preferences.chat_context_hotkeys = true;
        preferences.manual_hotkey_profile = false;
        self.apply(preferences);
        self.pending_chat_mode = Some(chat_on);
        self.chat_context = None;
        Ok(())
    }

    pub fn manual_hotkeys(&mut self) {
        let mut preferences = self.preferences.clone();
        preferences.chat_context_hotkeys = false;
        preferences.manual_hotkey_profile = true;
        self.apply(preferences);
        self.pending_chat_mode = None;
        self.chat_context = None;
    }

    pub fn sync_chat_context(&mut self, link: &crate::play::PlayLink) {
        let mut save = None;
        let chat_on = {
            let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
            if let Some(chat_on) = self.pending_chat_mode.take() {
                mailbox.chat.console.mode = if chat_on {
                    ComposerMode::On
                } else {
                    ComposerMode::Off
                };
            }
            let session = std::sync::Arc::as_ptr(&link.mailbox) as usize;
            let console = &mut mailbox.chat.console;
            match self.chat_mode_seen {
                Some((seen, _)) if seen != session => self.chat_mode_seen = None,
                _ => {}
            }
            match self.chat_mode_seen {
                // A new play session starts in the character's saved chat mode.
                None => {
                    if let Some(saved) = self.preferences.chat_mode {
                        console.mode = composer(saved);
                        console.auto_focus = false;
                    }
                    self.chat_mode_seen = Some((session, chosen(console.mode)));
                }
                Some((_, seen)) if chosen(console.mode) != seen => {
                    let mode = chosen(console.mode);
                    self.chat_mode_seen = Some((session, mode));
                    let saved = persisted(mode);
                    if self.preferences.chat_mode != Some(saved) {
                        save = Some(saved);
                    }
                }
                Some(_) => {}
            }
            // Chat On and an Auto input holding the focus use the chatOn hotkey set.
            console.can_send()
        };
        if let Some(saved) = save {
            let mut preferences = self.preferences.clone();
            preferences.chat_mode = Some(saved);
            self.apply(preferences);
        }
        if !self.preferences.chat_context_hotkeys || self.chat_context == Some(chat_on) {
            return;
        }
        let mut preferences = self.preferences.clone();
        match preferences.general_hotkey_preset(chat_on) {
            Ok(()) => {
                // A temporary composer mode is runtime state; it must not write the file per message.
                self.preferences = preferences;
                self.chat_context = Some(chat_on);
            }
            Err(error) => {
                self.feedback = format!("Chat preset conflicts with a custom binding: {error}")
            }
        }
    }
}
