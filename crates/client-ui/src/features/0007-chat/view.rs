use super::ChatPanel;
use oteryn_client_runtime::chat::ChatLine;
use std::fmt::Write;
#[must_use]
pub fn panel_text(panel: &ChatPanel) -> String {
    tab_text(panel, panel.console.tab)
}

/// A mirror exposes only text, with no composer or send action of its own.
#[must_use]
pub fn mirror_text(panel: &ChatPanel) -> Option<String> {
    if !panel.open {
        return None;
    }
    panel.console.mirror.map(|tab| tab_text(panel, tab))
}

fn tab_text(panel: &ChatPanel, tab: super::ConsoleTab) -> String {
    let show_messages = panel.console.shows_messages(tab);
    let mut text = String::from(tr!("Local Chat — Say / Whisper / Yell / Private\n"));
    if panel.console.presentation.visible[2] {
        text.push_str(&crate::chat_guild_room::panel_text(panel));
    }
    let presentation = &panel.console.presentation;
    for (index, line) in panel.lines.iter().enumerate().skip(
        panel
            .lines
            .len()
            .saturating_sub(usize::from(presentation.limit)),
    ) {
        if !presentation.shows(line) || !panel.console.filters.visible(tab, line) {
            continue;
        }
        if !panel.console.communication.visible(line) {
            continue;
        }
        if !show_messages {
            continue;
        }
        use crate::chat::ConsoleTab;
        if !matches!(
            (tab, line),
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
            && !matches!(&panel.guild, oteryn_client_runtime::chat_guild_room::GuildRoomState::Member { guild_id, .. } if *guild_id == line.guild_id)
        {
            continue;
        }
        text.push_str(&panel.console.metadata.prefix(panel.console.display, index));
        match line {
            ChatLine::Local {
                speaker_name,
                mode,
                text: message,
                ..
            } => {
                let speaker_name =
                    if *mode == oteryn_client_runtime::chat::ChatSpeechMode::MonsterSay {
                        speaker_name.clone()
                    } else {
                        panel
                            .console
                            .metadata
                            .name(panel.console.display, speaker_name)
                    };
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
                let speaker_name = panel
                    .console
                    .metadata
                    .name(panel.console.display, speaker_name);
                if presentation.mode_labels {
                    let _ = lwriteln!(
                        text,
                        "[Private] {speaker_name}: {message}",
                        message = message,
                        speaker_name = speaker_name
                    );
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
                let name = panel
                    .console
                    .metadata
                    .name(panel.console.display, &line.speaker_name);
                let _ = writeln!(text, "{label}{name}: {}", line.text);
            }
            ChatLine::Dropped => text.push_str(tr!("[Some chat lines were dropped]\n")),
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
                let speaker_name = panel
                    .console
                    .metadata
                    .name(panel.console.display, speaker_name);
                let _ = writeln!(text, "{label}{speaker_name}: {message}");
            }
            ChatLine::Room {
                room,
                speaker_name,
                text: message,
            } => {
                let name = panel
                    .console
                    .metadata
                    .name(panel.console.display, speaker_name);
                if presentation.mode_labels {
                    let _ = writeln!(text, "[{room:?}] {name}: {message}");
                } else {
                    let _ = writeln!(text, "{name}: {message}");
                }
            }
            ChatLine::PrivateChannel(line) => {
                let name = panel
                    .console
                    .metadata
                    .name(panel.console.display, &line.speaker_name);
                let label = if presentation.mode_labels {
                    "[Private channel] "
                } else {
                    ""
                };
                let _ = writeln!(text, "{label}{name}: {}", line.text);
            }
            _ => {}
        }
    }
    if show_messages && tab == crate::chat::ConsoleTab::All {
        text.push_str(
            &panel
                .console
                .notices
                .text(panel.console.filters, panel.console.display),
        );
    }
    if let Some(feedback) = &panel.feedback
        && presentation.feedback
        && panel.console.filters.information
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
