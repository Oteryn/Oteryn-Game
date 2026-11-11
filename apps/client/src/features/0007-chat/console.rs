use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, SharedMailbox, lock},
};
use oteryn_client_ui::chat::{ChannelEntry, CommunicationList, ComposerMode, ConsoleTab};
use std::path::Path;

impl PlayLink {
    /// PreviousChannel / NextChannel: cycle the console tabs.
    pub fn chat_channel_cycle(&self, forward: bool) {
        lock(&self.mailbox).chat.console.cycle(forward);
    }
    /// The rows of the channel list (Ctrl+O) the server currently offers.
    pub fn chat_channel_entries(&self) -> Vec<ChannelEntry> {
        let mailbox = lock(&self.mailbox);
        oteryn_client_ui::chat::channel_entries(
            mailbox.chat_private.list.as_ref(),
            mailbox.chat_public.rooms,
        )
    }
    /// The channel list as text, one row per channel.
    pub fn chat_channel_list_text(&self) -> String {
        let entries = self.chat_channel_entries();
        oteryn_client_ui::chat::channel_list_text(&entries, &lock(&self.mailbox).chat.console)
    }
    /// The rows of the channel list dialog: the server's channels, then the local tabs.
    pub fn chat_dialog_entries(&self) -> Vec<ChannelEntry> {
        let mailbox = lock(&self.mailbox);
        oteryn_client_ui::chat::dialog_entries(
            mailbox.chat_private.list.as_ref(),
            mailbox.chat_public.rooms,
        )
    }
    /// OpenChannelList (Ctrl+O): show the channel list dialog next to the console.
    pub fn chat_channel_dialog_open(&self) {
        lock(&self.mailbox).chat.console.dialog.show();
        self.open_chat();
    }
    /// The dialog's state and one caption per row.
    pub fn chat_channel_dialog(&self) -> (oteryn_client_ui::chat::ChannelDialog, Vec<String>) {
        let entries = self.chat_dialog_entries();
        let mailbox = lock(&self.mailbox);
        (
            mailbox.chat.console.dialog.clone(),
            oteryn_client_ui::chat::dialog_rows(&entries, &mailbox.chat.console),
        )
    }
    /// A row was selected in the dialog.
    pub fn chat_channel_dialog_select(&self, index: usize) {
        let rows = self.chat_dialog_entries().len();
        lock(&self.mailbox).chat.console.dialog.select(index, rows);
    }
    /// The private-message name box changed.
    pub fn chat_channel_dialog_name(&self, name: &str) {
        lock(&self.mailbox).chat.console.dialog.set_name(name);
    }
    pub fn chat_channel_dialog_cancel(&self) {
        lock(&self.mailbox).chat.console.dialog.hide();
    }
    /// Open what the dialog selected, or the private-message tab of the typed name.
    pub fn chat_channel_dialog_confirm(&self) -> bool {
        dialog_confirm(&mut lock(&self.mailbox), &self.commands)
    }
    /// OpenHelpChannel.
    pub fn chat_open_help(&self) {
        self.chat_channel_command(|mailbox| {
            oteryn_client_ui::chat::open_help(&mut mailbox.chat, &mut mailbox.chat_public)
        });
    }
    /// CloseCurrentChannel: private-message tabs close locally, rooms and private channels
    /// are left on the server.
    pub fn chat_close_current(&self) {
        self.chat_channel_command(|mailbox| {
            oteryn_client_ui::chat::close_current(
                &mut mailbox.chat,
                &mut mailbox.chat_public,
                &mut mailbox.chat_private,
            )
        });
    }
    /// Open the channel a channel-list row names.
    pub fn chat_open_entry(&self, entry: &ChannelEntry) {
        self.chat_channel_command(|mailbox| {
            oteryn_client_ui::chat::open_entry(
                &mut mailbox.chat,
                &mut mailbox.chat_public,
                &mut mailbox.chat_private,
                entry,
            )
        });
    }
    /// Open a private-message tab for a player name.
    pub fn chat_open_conversation(&self, name: &str) -> bool {
        oteryn_client_ui::chat::open_conversation(&mut lock(&self.mailbox).chat, name)
    }
    fn chat_channel_command(
        &self,
        command: impl FnOnce(&mut Mailbox) -> Option<oteryn_session::ChatIntent>,
    ) {
        channel_command(&mut lock(&self.mailbox), &self.commands, command);
    }
    pub fn chat_console_tab(&self, tab: ConsoleTab) {
        select(&mut lock(&self.mailbox), tab);
    }
    pub fn chat_console_drop(&self, tab: ConsoleTab, x: i32, width: i32) -> bool {
        drop_tab(&mut lock(&self.mailbox), tab, x, width)
    }
    pub fn chat_console_filter(&self) {
        toggle_filter(&mut lock(&self.mailbox));
    }
    pub fn chat_console_mode(&self) {
        cycle_mode(&mut lock(&self.mailbox));
    }
    pub fn chat_communication(
        &self,
        list: Option<CommunicationList>,
        name: &str,
        add: bool,
    ) -> bool {
        edit_lists(&mut lock(&self.mailbox), list, name, add)
    }
    pub fn chat_show_lists(&self) {
        show_lists(&mut lock(&self.mailbox));
    }
    pub fn chat_save_log(&self, path: &Path, append: bool) -> std::io::Result<()> {
        save_log(&self.mailbox, path, append)
    }
}
pub(in crate::play) fn select(mailbox: &mut Mailbox, tab: ConsoleTab) {
    mailbox.chat.console.select(tab);
}
pub(super) fn drop_tab(mailbox: &mut Mailbox, tab: ConsoleTab, x: i32, width: i32) -> bool {
    mailbox.chat.open && mailbox.chat.console.drop_tab(tab, x, width)
}
#[cfg(windows)]
pub(super) fn close(mailbox: &mut Mailbox) {
    mailbox.chat.open = false;
    mailbox.chat.console.close();
}
pub(in crate::play) fn toggle_filter(mailbox: &mut Mailbox) {
    mailbox.chat.console.show_messages = !mailbox.chat.console.show_messages;
}
pub(in crate::play) fn cycle_mode(mailbox: &mut Mailbox) {
    mailbox.chat.console.mode = match mailbox.chat.console.mode {
        ComposerMode::On => ComposerMode::Off,
        ComposerMode::Off => ComposerMode::Temporary,
        ComposerMode::Temporary => ComposerMode::On,
    };
}
pub(in crate::play) fn edit_lists(
    mailbox: &mut Mailbox,
    list: Option<CommunicationList>,
    name: &str,
    add: bool,
) -> bool {
    let changed = if let Some(list) = list {
        mailbox.chat.console.communication.edit(list, name, add)
    } else {
        [
            CommunicationList::Ignore,
            CommunicationList::Whitelist,
            CommunicationList::Blacklist,
        ]
        .into_iter()
        .all(|list| mailbox.chat.console.communication.edit(list, name, false))
    };
    if !changed {
        mailbox.chat.feedback =
            Some("Choose a valid player name; communication lists hold at most 200 names".into());
        return false;
    }
    mailbox.chat_private.communication = mailbox.chat.console.communication.clone();
    mailbox.chat_party_room.communication = mailbox.chat.console.communication.clone();
    show_lists(mailbox);
    true
}
/// Report the three communication lists in the chat feedback line.
pub(in crate::play) fn show_lists(mailbox: &mut Mailbox) {
    mailbox.chat.feedback = Some(lfmt!(
        "Ignore: {:?}; whitelist: {:?}; blacklist: {:?}",
        mailbox.chat.console.communication.ignored,
        mailbox.chat.console.communication.whitelist,
        mailbox.chat.console.communication.blacklist
    ));
}
pub(in crate::play) fn save_log(
    shared: &SharedMailbox,
    path: &Path,
    append: bool,
) -> std::io::Result<()> {
    let snapshot = lock(shared).chat.clone();
    let result = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path)
        .and_then(|mut file| oteryn_client_ui::chat::write_log(&snapshot, &mut file));
    lock(shared).chat.feedback = Some(match &result {
        Ok(()) => tr!("Channel log written").into(),
        Err(error) => lfmt!("Could not write channel log: {error}", error = error),
    });
    result
}
use crate::play::Outbox;
use oteryn_client_ui::chat::{ConsoleDisplay, console_request};
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn console_display(&self, display: ConsoleDisplay) -> Result<(), &'static str> {
        if !display.valid() {
            return Err("Choose a supported console font size");
        }
        let mut mailbox = lock(&self.mailbox);
        mailbox.chat.console.display = display;
        refresh(&mut mailbox, &self.commands);
        Ok(())
    }
}
pub(super) fn refresh(mailbox: &mut Mailbox, commands: &UnboundedSender<PlayCommand>) {
    if let Some(payload) = console_request(&mut mailbox.chat)
        && commands
            .send(PlayCommand::Feature(
                oteryn_client_runtime::raw_feature::FeatureCommand {
                    capability: 7,
                    kind: 6117,
                    payload,
                },
            ))
            .is_err()
    {
        mailbox.chat.console.metadata.pending = None;
    }
}
/// Send what a channel command asks for; a closed session leaves a feedback line instead.
pub(in crate::play) fn channel_command(
    mailbox: &mut Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    command: impl FnOnce(&mut Mailbox) -> Option<oteryn_session::ChatIntent>,
) {
    if let Some(intent) = command(mailbox)
        && commands.send(PlayCommand::ChatPublic(intent)).is_err()
    {
        mailbox.chat_public.pending = false;
        mailbox.chat_private.pending = false;
        mailbox.chat_private.pending_intent = None;
        mailbox.chat.console.focus = None;
        mailbox.chat.feedback = Some(tr!("The session has ended").into());
    }
}

/// Confirm the channel list dialog: open the selected row, or the private-message tab of the
/// typed name. The native dialog calls this with the mailbox it already holds.
pub(in crate::play) fn dialog_confirm(
    mailbox: &mut Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
) -> bool {
    use oteryn_client_ui::chat::DialogAction;
    let entries = oteryn_client_ui::chat::dialog_entries(
        mailbox.chat_private.list.as_ref(),
        mailbox.chat_public.rooms,
    );
    match mailbox.chat.console.dialog.confirm(&entries) {
        Some(DialogAction::Entry(entry)) => {
            channel_command(mailbox, commands, |mailbox| {
                oteryn_client_ui::chat::open_entry(
                    &mut mailbox.chat,
                    &mut mailbox.chat_public,
                    &mut mailbox.chat_private,
                    &entry,
                )
            });
            true
        }
        Some(DialogAction::Conversation(name)) => {
            oteryn_client_ui::chat::open_conversation(&mut mailbox.chat, &name)
        }
        None => false,
    }
}

impl Outbox {
    pub(in crate::play) fn console_events(&self, events: &[oteryn_session::SessionEvent]) {
        let received = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut mailbox = lock(&self.0);
        for event in events {
            match event {
                oteryn_session::SessionEvent::ChatLine(delta) => mailbox
                    .chat
                    .console
                    .metadata
                    .received(&delta.value, received),
                oteryn_session::SessionEvent::ServerMessage(delta) => {
                    use oteryn_client_ui::chat::NoticeClass;
                    let (class, text) = match &delta.value {
                        oteryn_session::ServerMessage::Advance(text) => {
                            (NoticeClass::Event, text.clone())
                        }
                        oteryn_session::ServerMessage::StatusText(text) => {
                            (NoticeClass::Status, text.clone())
                        }
                        oteryn_session::ServerMessage::Warning(text) => {
                            (NoticeClass::Status, format!("Warning: {text}"))
                        }
                    };
                    mailbox.chat.console.notices.push(class, &text, received);
                }
                oteryn_session::SessionEvent::ChatSnapshot(_) => {
                    mailbox.chat.console.metadata.reset_history()
                }
                _ => {}
            }
        }
    }

    pub(in crate::play) fn console_result(&self, result: &oteryn_session::GatedCommandResult) {
        let mut mailbox = lock(&self.0);
        let Some(names) = mailbox.chat.console.metadata.pending.take() else {
            return;
        };
        for name in &names {
            mailbox.chat.console.metadata.levels.remove(name);
        }
        if !matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        ) {
            return;
        }
        let Some(levels) = oteryn_client_runtime::chat::console::decode_levels(&result.payload)
            .filter(|levels| levels.len() == names.len())
        else {
            return;
        };
        for (name, level) in names.into_iter().zip(levels) {
            if let Some(level) = level {
                mailbox.chat.console.metadata.levels.insert(name, level);
            } else {
                mailbox.chat.console.metadata.levels.remove(&name);
            }
        }
    }
}

/// Cache the complete captions; ownership can change a label without adding a tab.
#[cfg(any(windows, test))]
pub(super) fn update_tab_labels(held: &mut Vec<String>, labels: &[String]) -> bool {
    if held == labels {
        return false;
    }
    held.clear();
    held.extend_from_slice(labels);
    true
}

/// Native editors use the same saved custom bindings as the game window.
#[cfg(any(windows, test))]
pub(super) fn native_key(
    link: &PlayLink,
    input: &mut crate::play::PreferenceInput,
    virtual_key: u32,
    modifiers: oteryn_input_actions::Modifiers,
    pressed: bool,
    repeat: bool,
) -> bool {
    use oteryn_input_actions::{ButtonState, KeyCode, NormalizedInputEvent};
    let code = match virtual_key {
        0x41..=0x5a => virtual_key - 0x41 + 4,
        0x31..=0x39 => virtual_key - 0x31 + 30,
        0x30 => 39,
        0x70..=0x7b => virtual_key - 0x70 + 58,
        0x09 => 43,
        0x20 => 44,
        _ => return false,
    };
    let Ok(code) = KeyCode::new(code as u16) else {
        return false;
    };
    let mut preferences = lock(&link.mailbox).local_preferences.clone();
    // The native editor handles text and editing itself. Only session utility bindings
    // are dispatched here; settings-only actions belong to the game/settings thread.
    preferences.custom_hotkeys.retain(|_, action| {
        matches!(
            action,
            crate::play::CustomAction::Utility {
                action: crate::play::UtilityAction::CloseCurrentChannel
                    | crate::play::UtilityAction::NextChannel
                    | crate::play::UtilityAction::PreviousChannel
                    | crate::play::UtilityAction::OpenChannelList
                    | crate::play::UtilityAction::OpenHelpChannel
                    | crate::play::UtilityAction::ToggleLensHelp
            }
        )
    });
    preferences.action_keys.0.clear();
    let bound = preferences.custom_hotkeys.keys().any(|chord| {
        crate::play::feature_local_reference_preferences::settings::parse_action_chord(chord)
            == Some((code, modifiers))
    });
    let event = NormalizedInputEvent::Key {
        code,
        modifiers,
        repeat,
        state: if pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        },
    };
    let Ok(routed) = input.route_chat(&preferences, &[event], true) else {
        return false;
    };
    let actions = input.take_custom_actions();
    for action in &actions {
        if let crate::play::CustomAction::Utility { action } = action {
            link.utility_action(*action);
        }
    }
    (bound && routed.is_empty()) || !actions.is_empty()
}
