use super::{ChatPanel, Mode};
use oteryn_client_runtime::chat::{ChatIntent, ChatSpeechMode};
pub fn request(panel: &mut ChatPanel, mode: Mode, recipient: &str) -> Option<ChatIntent> {
    if panel.console.mode == super::ComposerMode::Off {
        panel.feedback = Some("Chat input is off".into());
        return None;
    }
    if !panel.selected || panel.pending {
        panel.feedback = Some("Chat unavailable or waiting for the server".into());
        return None;
    }
    if panel.text.trim().is_empty() || panel.text.len() > 1_020 || panel.text.contains('\0') {
        panel.feedback = Some("Enter a message of at most 1,020 bytes".into());
        return None;
    }
    let mode = match panel.console.tab {
        super::ConsoleTab::Private => Mode::Private,
        super::ConsoleTab::Guild => Mode::Guild,
        _ => mode,
    };
    let routed = match panel.console.tab {
        super::ConsoleTab::Public(room) => Some(ChatIntent::Room {
            room,
            text: panel.text.clone(),
        }),
        super::ConsoleTab::Party => Some(ChatIntent::Party {
            text: panel.text.clone(),
        }),
        super::ConsoleTab::Channel(channel_id) => Some(ChatIntent::PrivateChannel(
            oteryn_client_runtime::chat_private_channel::PrivateChannelIntent::Say {
                channel_id,
                text: panel.text.clone(),
            },
        )),
        _ => None,
    };
    let intent = if let Some(intent) = routed {
        intent
    } else if mode == Mode::Guild {
        let intent = crate::chat_guild_room::request(panel)?;
        if panel.console.mode == super::ComposerMode::Temporary {
            panel.console.mode = super::ComposerMode::Off;
        }
        return Some(intent);
    } else if mode == Mode::Private {
        let name = recipient.trim();
        if name.is_empty() || name.len() > 120 || name.contains('\0') {
            panel.feedback = Some("Enter a player name of at most 120 bytes".into());
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
    if panel.console.mode == super::ComposerMode::Temporary {
        panel.console.mode = super::ComposerMode::Off;
    }
    panel.pending = true;
    panel.feedback = None;
    Some(intent)
}
