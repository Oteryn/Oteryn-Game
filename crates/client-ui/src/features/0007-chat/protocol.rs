use super::display::speaker;
use super::{ChatPanel, Mode};
use oteryn_client_runtime::chat::{ChatIntent, ChatSpeechMode};
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
