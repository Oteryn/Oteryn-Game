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
        #[cfg(bevy_shell)]
        if self.preference_transfer_pending() {
            return;
        }
        let mut save = None;
        let local_channels;
        let mut reopen = None;
        let mut public_rooms = None;
        let conversations;
        let guild_channel;
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
            let session = (
                std::sync::Arc::as_ptr(&link.mailbox) as usize,
                mailbox.chat.scope,
            );
            let rooms = mailbox.chat_public.rooms;
            let ready = mailbox.chat.selected
                && !mailbox.chat_public.pending
                && mailbox.chat.console.focus.is_none();
            let public = mailbox.chat_public.selected;
            let guild = oteryn_client_ui::chat_guild_room::available(&mailbox.chat);
            let console = &mut mailbox.chat.console;
            if self.chat_mode_seen.is_some_and(|(seen, _)| seen != session) {
                self.chat_mode_seen = None;
            }
            use oteryn_client_ui::chat::ChannelTab;
            let tabs = [ChannelTab::Loot, ChannelTab::ServerLog, ChannelTab::Npc];
            if self.chat_mode_seen.is_none() {
                self.room_retry = [None; 8];
                self.unconfirmed_rooms = self.preferences.console.public_rooms;
                let keep = |tab: &ChannelTab| match tab {
                    ChannelTab::Conversation(name) => {
                        self.preferences.console.conversations.contains(name)
                    }
                    ChannelTab::Guild => self.preferences.console.guild_channel,
                    _ => tabs
                        .iter()
                        .position(|local| local == tab)
                        .is_none_or(|i| self.preferences.console.local_channels[i]),
                };
                let closed: Vec<_> = console
                    .channels
                    .iter()
                    .filter(|tab| !keep(tab))
                    .cloned()
                    .collect();
                for tab in closed {
                    console.drop_channel(&tab);
                }
                self.restore_guild_channel = self.preferences.console.guild_channel;
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
            if guild
                && std::mem::take(&mut self.restore_guild_channel)
                && !console.channels.contains(&ChannelTab::Guild)
            {
                console.channels.push(ChannelTab::Guild);
            }
            guild_channel =
                self.restore_guild_channel || console.channels.contains(&ChannelTab::Guild);
            if ready {
                for (index, room) in ChatRoom::ALL.into_iter().enumerate() {
                    if rooms.contains(room) {
                        self.unconfirmed_rooms[index] = false;
                        self.room_retry[index] = None;
                    } else if (public || !room.requires_public_channels())
                        && self.unconfirmed_rooms[index]
                        && self.room_retry[index]
                            .is_none_or(|last| last.elapsed() >= std::time::Duration::from_secs(5))
                    {
                        self.room_retry[index] = Some(std::time::Instant::now());
                        reopen = Some(room);
                        break;
                    }
                }
                if reopen.is_none() {
                    public_rooms = Some(std::array::from_fn(|i| {
                        self.unconfirmed_rooms[i] || rooms.contains(ChatRoom::ALL[i])
                    }));
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
            || self.preferences.console.guild_channel != guild_channel
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
            preferences.console.guild_channel = guild_channel;
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
