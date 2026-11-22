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
        let local_channels;
        let mut reopen = None;
        let mut public_rooms = None;
        let conversations;
        use oteryn_session::ChatRoom;
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
            let rooms = mailbox.chat_public.rooms;
            let ready = mailbox.chat.selected
                && !mailbox.chat_public.pending
                && mailbox.chat.console.focus.is_none();
            let public = mailbox.chat_public.selected;
            let console = &mut mailbox.chat.console;
            match self.chat_mode_seen {
                Some((seen, _)) if seen != session => self.chat_mode_seen = None,
                _ => {}
            }
            use oteryn_client_ui::chat::ChannelTab;
            let tabs = [ChannelTab::Loot, ChannelTab::ServerLog, ChannelTab::Npc];
            if self.chat_mode_seen.is_none() {
                self.restoring_rooms = self.preferences.console.public_rooms;
                for name in &self.preferences.console.conversations {
                    let tab = ChannelTab::Conversation(name.clone());
                    if !console.channels.contains(&tab) && console.communication.allows(name) {
                        console.channels.push(tab);
                    }
                }
                for (tab, open) in tabs.iter().zip(self.preferences.console.local_channels) {
                    if open && !console.channels.contains(tab) {
                        console.channels.push(tab.clone());
                    }
                }
            }
            if ready {
                for (index, room) in ChatRoom::ALL.into_iter().enumerate() {
                    if std::mem::take(&mut self.restoring_rooms[index])
                        && !rooms.contains(room)
                        && (public || !room.requires_public_channels())
                    {
                        reopen = Some(room);
                        break;
                    }
                }
                if reopen.is_none() {
                    public_rooms = Some(ChatRoom::ALL.map(|room| rooms.contains(room)));
                }
            }
            let names = console
                .channels
                .iter()
                .filter_map(|tab| match tab {
                    ChannelTab::Conversation(name) => Some(name),
                    _ => None,
                })
                .take(128);
            conversations = (!names.clone().eq(&self.preferences.console.conversations))
                .then(|| names.cloned().collect());
            local_channels = tabs.map(|tab| console.channels.contains(&tab));
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
        if let Some(room) = reopen {
            link.chat_open_entry(&oteryn_client_ui::chat::ChannelEntry::Room(room));
        }
        if save.is_some()
            || self.preferences.console.local_channels != local_channels
            || conversations.is_some()
            || public_rooms.is_some_and(|rooms| rooms != self.preferences.console.public_rooms)
        {
            let mut preferences = self.preferences.clone();
            if let Some(names) = conversations {
                preferences.console.conversations = names;
            }
            if let Some(rooms) = public_rooms {
                preferences.console.public_rooms = rooms;
            }
            preferences.console.local_channels = local_channels;
            if let Some(saved) = save {
                preferences.chat_mode = Some(saved);
            }
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
