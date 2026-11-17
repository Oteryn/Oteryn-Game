use super::ChatPanel;
use oteryn_client_runtime::chat_public_channels::{ChatLine, ChatRoom};
use std::fmt::Write;

#[must_use]
pub fn room_name(room: ChatRoom) -> &'static str {
    match room {
        ChatRoom::World => tr!("World"),
        ChatRoom::English => tr!("English"),
        ChatRoom::Help => tr!("Help"),
        ChatRoom::Advertising => tr!("Advertising"),
        ChatRoom::AdvertisingRookgaard => tr!("Advertising-Rookgaard"),
        ChatRoom::Tutor => tr!("Tutor"),
        ChatRoom::Gamemaster => tr!("Gamemaster"),
        ChatRoom::GuildLeaders => tr!("Guild Leaders"),
    }
}

#[must_use]
pub fn line_text(line: &ChatLine) -> Option<String> {
    match line {
        ChatLine::Room {
            room,
            speaker_name,
            text,
        } => Some(format!("[{}] {speaker_name}: {text}", room_name(*room))),
        ChatLine::Dropped => Some(tr!("[Some chat lines were dropped]").into()),
        _ => None,
    }
}

#[must_use]
pub fn panel_text(panel: &ChatPanel) -> String {
    let mut text = String::from(tr!("Public Channels\n"));
    for room in ChatRoom::ALL {
        let state = if panel.rooms.contains(room) {
            tr!("open")
        } else {
            tr!("closed")
        };
        let _ = writeln!(text, "{}: {state}", room_name(room));
    }
    let presentation = &panel.console.presentation;
    for line in panel.lines.iter().skip(
        panel
            .lines
            .len()
            .saturating_sub(usize::from(presentation.limit)),
    ) {
        if !presentation.shows(line) {
            continue;
        }
        if !panel.console.communication.visible(line) {
            continue;
        }
        if !presentation.mode_labels
            && let ChatLine::Room {
                speaker_name,
                text: message,
                ..
            } = line
        {
            let _ = writeln!(text, "{speaker_name}: {message}");
        } else if let Some(line) = line_text(line) {
            let _ = writeln!(text, "{line}");
        }
    }
    if let Some(feedback) = &panel.feedback
        && presentation.feedback
    {
        let _ = writeln!(text, "{feedback}");
    }
    text
}
