use super::{ChatPanel, Mode};
use oteryn_client_runtime::chat::{ChatIntent, ChatSpeechMode};
pub fn request(panel: &mut ChatPanel, mode: Mode, recipient: &str) -> Option<ChatIntent> {
    if !panel.selected || panel.pending || !panel.console.can_send() {
        panel.feedback = Some("Chat unavailable or waiting for the server".into());
        return None;
    }
    if panel.text.trim().is_empty() || panel.text.len() > 1_020 || panel.text.contains('\0') {
        panel.feedback = Some("Enter a message of at most 1,020 bytes".into());
        return None;
    }
    let intent = if mode == Mode::Guild {
        return crate::chat_guild_room::request(panel);
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
    panel.pending = true;
    panel.feedback = None;
    Some(intent)
}
