use super::display::speaker;
use super::{ChatPanel, Mode};
use oteryn_client_runtime::chat::{ChatIntent, ChatSpeechMode};
use oteryn_client_runtime::chat_public_channels::ChatRoom;
pub fn request(panel: &mut ChatPanel, mode: Mode, recipient: &str) -> Option<ChatIntent> {
    if !panel.selected || panel.pending || !panel.console.can_send() {
        panel.feedback = Some(tr!("Chat unavailable or waiting for the server").into());
        return None;
    }
    if panel.text.trim().is_empty() || panel.text.len() > 1_020 || panel.text.contains('\0') {
        panel.feedback = Some(tr!("Enter a message of at most 1,020 bytes").into());
        return None;
    }
    let intent = if mode == Mode::Guild {
        return crate::chat_guild_room::request(panel);
    } else if mode == Mode::Private {
        let name = recipient.trim();
        if name.is_empty() || name.len() > 120 || name.contains('\0') {
            panel.feedback = Some(tr!("Enter a player name of at most 120 bytes").into());
            return None;
        }
        ChatIntent::Private {
            recipient_name: name.into(),
            text: panel.text.clone(),
        }
    } else {
        let mode = match mode {
            Mode::Whisper => ChatSpeechMode::Whisper,
            Mode::Yell => ChatSpeechMode::Yell,
            _ => ChatSpeechMode::Say,
        };
        ChatIntent::Say {
            mode,
            text: panel.text.clone(),
        }
    };
    panel.pending = true;
    panel.feedback = None;
    Some(intent)
}

/// Send the composer text to the selected channel tab; `None` while a fixed tab is selected.
pub fn channel_request(panel: &mut ChatPanel) -> Option<ChatIntent> {
    use super::ChannelTab;
    let channel = panel.console.current.clone()?;
    if !panel.selected || panel.pending || !panel.console.can_send() {
        panel.feedback = Some(tr!("Chat unavailable or waiting for the server").into());
        return None;
    }
    if panel.text.trim().is_empty() || panel.text.len() > 1_020 || panel.text.contains('\0') {
        panel.feedback = Some(tr!("Enter a message of at most 1,020 bytes").into());
        return None;
    }
    let text = panel.text.clone();
    let intent = match channel {
        ChannelTab::Room(room) => ChatIntent::Room { room, text },
        ChannelTab::Private(channel_id) => ChatIntent::PrivateChannel(
            oteryn_client_runtime::chat_private_channel::PrivateChannelIntent::Say {
                channel_id,
                text,
            },
        ),
        ChannelTab::Conversation(recipient_name) => ChatIntent::Private {
            recipient_name,
            text,
        },
    };
    panel.pending = true;
    panel.feedback = None;
    Some(intent)
}

type Private = crate::chat_private_channel::ChatPanel;

fn open_room(chat: &mut ChatPanel, public: &mut ChatPanel, room: ChatRoom) -> Option<ChatIntent> {
    use super::{ChannelTab, PendingFocus};
    use crate::chat_public_channels::{Intent, request};
    let tab = ChannelTab::Room(room);
    if public.rooms.contains(room) {
        chat.console.open_channel(tab);
        return None;
    }
    let command = request(public, Intent::Open(room));
    if command.is_some() {
        chat.console.focus = Some(PendingFocus::Tab(tab));
    } else {
        chat.feedback = public.feedback.clone();
    }
    command
}

/// OpenHelpChannel: select the Help tab, asking the server to open the room first if needed.
pub fn open_help(chat: &mut ChatPanel, public: &mut ChatPanel) -> Option<ChatIntent> {
    open_room(chat, public, ChatRoom::Help)
}

/// Open the channel a channel-list row names. Private-channel tabs open once the server
/// accepted the join (see [`private_result`]).
pub fn open_entry(
    chat: &mut ChatPanel,
    public: &mut ChatPanel,
    private: &mut Private,
    entry: &super::ChannelEntry,
) -> Option<ChatIntent> {
    use super::{ChannelEntry, PendingFocus};
    use crate::chat_private_channel::{Intent, request};
    let (id, intent) = match entry {
        ChannelEntry::Room(room) => return open_room(chat, public, *room),
        ChannelEntry::CreateOwn => (None, Intent::Create),
        ChannelEntry::Own(id) | ChannelEntry::Invitation { channel_id: id, .. } => {
            (Some(*id), Intent::Join)
        }
    };
    if id.is_some() {
        private.channel = id;
    }
    let command = request(private, intent).map(ChatIntent::PrivateChannel);
    if command.is_some() && id.is_none() {
        chat.console.focus = Some(PendingFocus::OwnChannel);
    } else if command.is_none() {
        chat.feedback = private.feedback.clone();
    }
    command
}

/// Open a private-message tab. The name is the recipient of its lines; no wire call is needed.
pub fn open_conversation(chat: &mut ChatPanel, name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || name.len() > 120 || name.contains('\0') {
        chat.feedback = Some(tr!("Enter a player name of at most 120 bytes").into());
        return false;
    }
    chat.console
        .open_channel(super::ChannelTab::Conversation(name.into()));
    true
}

/// CloseCurrentChannel: leave or close the selected channel tab. Fixed tabs stay.
pub fn close_current(
    chat: &mut ChatPanel,
    public: &mut ChatPanel,
    private: &mut Private,
) -> Option<ChatIntent> {
    use super::ChannelTab;
    match chat.console.current.clone()? {
        ChannelTab::Room(room) => {
            // The tab disappears when the server reports the room closed.
            let command = crate::chat_public_channels::request(
                public,
                crate::chat_public_channels::Intent::Close(room),
            );
            if command.is_none() {
                chat.feedback = public.feedback.clone();
            }
            command
        }
        ChannelTab::Private(id) => {
            private.channel = Some(id);
            let command = crate::chat_private_channel::request(
                private,
                crate::chat_private_channel::Intent::Leave,
            )
            .map(ChatIntent::PrivateChannel);
            if command.is_none() {
                chat.feedback = private.feedback.clone();
            }
            command
        }
        ChannelTab::Conversation(_) => {
            chat.console.close_current();
            None
        }
    }
}

/// The server's answer to a private-channel command: an accepted join opens (and selects)
/// the tab, an accepted leave closes it, a refusal forgets a pending own-channel focus.
pub fn private_result(
    chat: &mut ChatPanel,
    intent: Option<crate::chat_private_channel::Intent>,
    channel: Option<u16>,
    outcome: &oteryn_client_runtime::chat_private_channel::ChatOutcome,
) {
    use super::ChannelTab;
    use crate::chat_private_channel::Intent;
    use oteryn_client_runtime::chat_private_channel::ChatDisposition;
    if outcome.disposition != ChatDisposition::Ok {
        chat.console.focus = None;
        return;
    }
    match (intent, channel) {
        (Some(Intent::Join), Some(id)) => chat.console.open_channel(ChannelTab::Private(id)),
        (Some(Intent::Leave), Some(id)) => chat.console.drop_channel(&ChannelTab::Private(id)),
        _ => {}
    }
}

pub fn console_request(panel: &mut super::ChatPanel) -> Option<Vec<u8>> {
    if !panel.selected || !panel.console.display.levels || panel.console.metadata.pending.is_some()
    {
        return None;
    }
    let mut names = Vec::new();
    for line in panel.lines.iter().rev() {
        if let Some(name) = speaker(line)
            && !names.iter().any(|n| n == name)
        {
            names.push(name.to_owned());
        }
    }
    if names.is_empty() {
        return None;
    }
    // Rotate through the bounded retained log, including senders above the visible tail.
    let start = panel.console.metadata.next_names % names.len();
    let names: Vec<String> = names
        .iter()
        .cycle()
        .skip(start)
        .take(names.len().min(32))
        .cloned()
        .collect();
    panel.console.metadata.next_names = start + names.len();
    let payload = oteryn_client_runtime::chat::console::encode_names(&names)?;
    panel.console.metadata.pending = Some(names);
    Some(payload)
}
