use super::ChatPanel;
use oteryn_client_runtime::chat::ChatLine;
use std::fmt::Write;
#[must_use]
pub fn panel_text(panel: &ChatPanel) -> String {
    let mut text = String::from("Local Chat — Say / Whisper / Yell / Private\n");
    if panel.console.presentation.visible[2] {
        text.push_str(&crate::chat_guild_room::panel_text(panel));
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
                if presentation.mode_labels {
                    let _ = writeln!(text, "[{mode:?}] {speaker_name}: {message}");
                } else {
                    let _ = writeln!(text, "{speaker_name}: {message}");
                }
            }
            ChatLine::Private {
                speaker_name,
                text: message,
            } => {
                if presentation.mode_labels {
                    let _ = writeln!(text, "[Private] {speaker_name}: {message}");
                } else {
                    let _ = writeln!(text, "{speaker_name}: {message}");
                }
            }
            ChatLine::Guild(line)
                if matches!(
                    &panel.guild,
                    oteryn_client_runtime::chat_guild_room::GuildRoomState::Member { guild_id, .. }
                        if *guild_id == line.guild_id
                ) =>
            {
                let label = if presentation.mode_labels {
                    "[Guild] "
                } else {
                    ""
                };
                let _ = writeln!(text, "{label}{}: {}", line.speaker_name, line.text);
            }
            ChatLine::Dropped => text.push_str("[Some chat lines were dropped]\n"),
            ChatLine::Party {
                speaker_name,
                text: message,
                ..
            } => {
                let label = if presentation.mode_labels {
                    "[Party] "
                } else {
                    ""
                };
                let _ = writeln!(text, "{label}{speaker_name}: {message}");
            }
            _ => {}
        }
    }
    if let Some(feedback) = &panel.feedback
        && presentation.feedback
    {
        let _ = writeln!(text, "{feedback}");
    }
    text
}

/// Export exactly the selected console projection. The caller owns file mode and path.
pub fn write_log(panel: &ChatPanel, writer: &mut impl std::io::Write) -> std::io::Result<()> {
    writer.write_all(panel_text(panel).as_bytes())?;
    writer.flush()
}
