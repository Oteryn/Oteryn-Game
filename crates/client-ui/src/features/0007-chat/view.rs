use super::ChatPanel;
use oteryn_client_runtime::chat::ChatLine;
use std::fmt::Write;
#[must_use]
pub fn panel_text(panel: &ChatPanel) -> String {
    let mut text = String::from("Local Chat — Say / Whisper / Yell / Private\n");
    for line in &panel.lines {
        match line {
            ChatLine::Local {
                speaker_name,
                mode,
                text: message,
                ..
            } => {
                let _ = writeln!(text, "[{mode:?}] {speaker_name}: {message}");
            }
            ChatLine::Private {
                speaker_name,
                text: message,
            } => {
                let _ = writeln!(text, "[Private] {speaker_name}: {message}");
            }
            ChatLine::Dropped => text.push_str("[Some chat lines were dropped]\n"),
            _ => {}
        }
    }
    if let Some(feedback) = &panel.feedback {
        let _ = writeln!(text, "{feedback}");
    }
    text
}
