//! Default movement presets follow the existing console's on/off/temporary composer.
use super::*;
use oteryn_client_ui::chat::ComposerMode;

impl SettingsState {
    pub fn apply_chat_preset(&mut self, chat_on: bool) -> Result<(), PreferenceError> {
        let mut preferences = self.preferences.clone();
        preferences.general_hotkey_preset(chat_on)?;
        preferences.chat_context_hotkeys = true;
        self.apply(preferences);
        self.pending_chat_mode = Some(chat_on);
        self.chat_context = None;
        Ok(())
    }

    pub fn manual_hotkeys(&mut self) {
        let mut preferences = self.preferences.clone();
        preferences.chat_context_hotkeys = false;
        self.apply(preferences);
        self.pending_chat_mode = None;
        self.chat_context = None;
    }

    pub fn sync_chat_context(&mut self, link: &crate::play::PlayLink) {
        let chat_on = {
            let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
            if let Some(chat_on) = self.pending_chat_mode.take() {
                mailbox.chat.console.mode = if chat_on { ComposerMode::On } else { ComposerMode::Off };
            }
            mailbox.chat.console.mode != ComposerMode::Off
        };
        if !self.preferences.chat_context_hotkeys || self.chat_context == Some(chat_on) { return; }
        let mut preferences = self.preferences.clone();
        match preferences.general_hotkey_preset(chat_on) {
            Ok(()) => {
                // A temporary composer mode is runtime state; it must not write the file per message.
                self.preferences = preferences;
                self.chat_context = Some(chat_on);
            }
            Err(error) => self.feedback = format!("Chat preset conflicts with a custom binding: {error}"),
        }
    }
}
