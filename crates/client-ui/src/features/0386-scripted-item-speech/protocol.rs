use oteryn_client_runtime::chat::{ChatLine, ChatSpeechMode};

/// Project validated server-owned speech through the existing native chat panel.
/// The session owns capability validation; ambient publication still requires 288.
#[must_use]
pub fn line_text(line: &ChatLine) -> Option<String> {
    match line {
        ChatLine::Local {
            speaker_name,
            mode: ChatSpeechMode::MonsterSay,
            text,
            ..
        } => Some(format!("[MonsterSay] {speaker_name}: {text}\n")),
        _ => None,
    }
}
