use super::ChatPanel;
use oteryn_client_runtime::chat::ChatLine;
use std::fmt::Write;
#[must_use]
pub fn panel_text(panel: &ChatPanel) -> String {
    let mut text = String::from("Local Chat — Say / Whisper / Yell / Private\n");
    text.push_str(&crate::chat_guild_room::panel_text(panel));
    for (index, line) in panel.lines.iter().enumerate() {
        if !panel.console.communication.visible(line) {
            continue;
        }
        if !panel.console.filters.visible(panel.console.tab, line) {
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
                | (ConsoleTab::Private, ChatLine::PrivateChannel(_))
                | (ConsoleTab::Guild, ChatLine::Guild(_))
                | (_, ChatLine::Dropped)
        ) {
            continue;
        }
        if let ChatLine::Guild(line) = line
            && !matches!(&panel.guild, oteryn_client_runtime::chat_guild_room::GuildRoomState::Member {guild_id,..} if *guild_id == line.guild_id)
        {
            continue;
        }
        let prefix = panel.console.metadata.prefix(panel.console.display, index);
        text.push_str(&prefix);
        match line {
            ChatLine::Local {
                speaker_name,
                mode,
                text: message,
                ..
            } => {
                let name = if *mode == oteryn_client_runtime::chat::ChatSpeechMode::MonsterSay {
                    speaker_name.clone()
                } else {
                    panel
                        .console
                        .metadata
                        .name(panel.console.display, speaker_name)
                };
                let _ = writeln!(text, "[{mode:?}] {}: {message}", name);
            }
            ChatLine::Private {
                speaker_name,
                text: message,
            } => {
                let _ = writeln!(
                    text,
                    "[Private] {}: {message}",
                    panel
                        .console
                        .metadata
                        .name(panel.console.display, speaker_name)
                );
            }
            ChatLine::Guild(line)
                if matches!(
                    &panel.guild,
                    oteryn_client_runtime::chat_guild_room::GuildRoomState::Member { guild_id, .. }
                        if *guild_id == line.guild_id
                ) =>
            {
                let _ = writeln!(
                    text,
                    "[Guild] {}: {}",
                    panel
                        .console
                        .metadata
                        .name(panel.console.display, &line.speaker_name),
                    line.text
                );
            }
            ChatLine::Room {
                room,
                speaker_name,
                text: message,
            } => {
                let _ = writeln!(
                    text,
                    "[{room:?}] {}: {message}",
                    panel
                        .console
                        .metadata
                        .name(panel.console.display, speaker_name)
                );
            }
            ChatLine::Party {
                speaker_name,
                text: message,
            } => {
                let _ = writeln!(
                    text,
                    "[Party] {}: {message}",
                    panel
                        .console
                        .metadata
                        .name(panel.console.display, speaker_name)
                );
            }
            ChatLine::PrivateChannel(line) => {
                let _ = writeln!(
                    text,
                    "[Private channel] {}: {}",
                    panel
                        .console
                        .metadata
                        .name(panel.console.display, &line.speaker_name),
                    line.text
                );
            }
            ChatLine::Dropped => text.push_str("[Some chat lines were dropped]\n"),
            _ => {}
        }
    }
    if panel.console.show_messages && panel.console.tab == crate::chat::ConsoleTab::All {
        text.push_str(
            &panel
                .console
                .notices
                .text(panel.console.filters, panel.console.display),
        );
    }
    if panel.console.filters.information
        && let Some(feedback) = &panel.feedback
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
