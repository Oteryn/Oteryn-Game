use super::ChatPanel;
use oteryn_client_runtime::chat::ChatLine;
use std::fmt::Write;
#[must_use]
pub fn panel_text(panel: &ChatPanel) -> String {
    let mut text = String::from("Local Chat — Say / Whisper / Yell / Private\n");
    text.push_str(&crate::chat_guild_room::panel_text(panel));
    for line in &panel.lines {
        if !panel.console.show_messages {
            continue;
        }
        use crate::chat::ConsoleTab;
        if !matches!(
            (panel.console.tab, line),
            (ConsoleTab::All, _)
                | (ConsoleTab::Local, ChatLine::Local { .. })
                | (ConsoleTab::Private, ChatLine::Private { .. })
                | (ConsoleTab::Guild, ChatLine::Guild(_))
                | (_, ChatLine::Dropped)
        ) {
            continue;
        }
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
            ChatLine::Guild(line)
                if matches!(
                    &panel.guild,
                    oteryn_client_runtime::chat_guild_room::GuildRoomState::Member { guild_id, .. }
                        if *guild_id == line.guild_id
                ) =>
            {
                let _ = writeln!(text, "[Guild] {}: {}", line.speaker_name, line.text);
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

/// Export exactly the selected console projection. The caller owns file mode and path.
pub fn write_log(panel: &ChatPanel, writer: &mut impl std::io::Write) -> std::io::Result<()> {
    writer.write_all(panel_text(panel).as_bytes())?;
    writer.flush()
}
