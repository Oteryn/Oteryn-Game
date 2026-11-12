use super::ChatPanel;
use oteryn_client_runtime::chat::ChatLine;
use std::fmt::Write;
#[must_use]
pub fn panel_text(panel: &ChatPanel) -> String {
    tab_text(panel, panel.console.tab, panel.console.current.as_ref())
}

/// A mirror exposes only text, with no composer or send action of its own.
#[must_use]
pub fn mirror_text(panel: &ChatPanel) -> Option<String> {
    if !panel.open {
        return None;
    }
    panel.console.mirror.map(|tab| tab_text(panel, tab, None))
}

fn tab_text(
    panel: &ChatPanel,
    tab: super::ConsoleTab,
    channel: Option<&super::ChannelTab>,
) -> String {
    // A channel tab filters like the Private tab: it is not the combined All view.
    let tab = if channel.is_some() {
        super::ConsoleTab::Private
    } else {
        tab
    };
    if let Some(local) = channel.filter(|channel| channel.receive_only()) {
        return local_tab_text(panel, local);
    }
    let show_messages = panel.console.shows_messages(tab);
    let mut text = String::from(tr!("Local Chat — Say / Whisper / Yell / Private\n"));
    if let Some(super::ChannelTab::Private(id)) = channel
        && let Some(names) = panel.console.participants.get(id)
    {
        let _ = writeln!(
            text,
            "{}",
            crate::chat_channel_participants::participants_text(names)
        );
    }
    if channel.is_none() && panel.console.presentation.visible[2] {
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
        let recipient = panel.recipients.get(index).and_then(|name| name.as_deref());
        if let Some(channel) = channel {
            if !channel_line(channel, line, recipient) {
                continue;
            }
        } else if !matches!(
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
                if let Some(recipient) = recipient {
                    // Capability 493: the own echo reads as sent to its recipient.
                    let _ = writeln!(
                        text,
                        "{}",
                        crate::chat_private_echo::echo_text(recipient, message)
                    );
                } else if presentation.mode_labels {
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
    if let Some(super::ChannelTab::Private(id)) = channel
        && show_messages
        && panel.console.show_server_messages
    {
        for notice in panel
            .console
            .channel_notices
            .iter()
            .filter(|notice| notice.channel_id == *id)
        {
            let _ = writeln!(
                text,
                "{}",
                crate::chat_channel_participants::notice_text(notice)
            );
        }
    }
    if channel.is_none()
        && show_messages
        && panel.console.show_server_messages
        && tab == crate::chat::ConsoleTab::All
    {
        text.push_str(&panel.console.notices.scoped_text(
            panel.console.filters,
            panel.console.display,
            super::NoticeScope::Combined {
                loot_tab_open: panel.console.channels.contains(&super::ChannelTab::Loot),
            },
        ));
    }
    if let Some(feedback) = &panel.feedback
        && presentation.feedback
        && panel.console.filters.information
    {
        let _ = writeln!(text, "{feedback}");
    }
    text
}

/// The Loot, Server Log and NPC tabs are client-local views over messages the client already
/// received: loot results, server notices and the latest NPC conversation.
fn local_tab_text(panel: &ChatPanel, tab: &super::ChannelTab) -> String {
    use super::{ChannelTab, NoticeScope};
    let console = &panel.console;
    let mut text = match tab {
        ChannelTab::Loot | ChannelTab::ServerLog if !console.show_server_messages => String::new(),
        ChannelTab::Loot => {
            console
                .notices
                .scoped_text(console.filters, console.display, NoticeScope::LootOnly)
        }
        ChannelTab::ServerLog => console.notices.scoped_text(
            console.filters,
            console.display,
            NoticeScope::Combined {
                loot_tab_open: console.channels.contains(&ChannelTab::Loot),
            },
        ),
        _ => console
            .npc_lines
            .iter()
            .fold(String::new(), |mut text, line| {
                let _ = writeln!(text, "{line}");
                text
            }),
    };
    if let Some(feedback) = &panel.feedback
        && console.presentation.feedback
        && console.filters.information
    {
        let _ = writeln!(text, "{feedback}");
    }
    text
}

/// A channel tab shows its own lines; a dropped-lines marker belongs to every tab.
fn channel_line(channel: &super::ChannelTab, line: &ChatLine, recipient: Option<&str>) -> bool {
    use super::ChannelTab;
    match (channel, line) {
        (ChannelTab::Room(room), ChatLine::Room { room: line, .. }) => room == line,
        (ChannelTab::Private(id), ChatLine::PrivateChannel(line)) => *id == line.channel_id,
        (ChannelTab::Conversation(name), ChatLine::Private { .. }) => {
            crate::chat_private_echo::in_conversation(name, line, recipient)
        }
        (_, ChatLine::Dropped) => true,
        _ => false,
    }
}

/// One row of the channel list dialog (Ctrl+O).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelEntry {
    Room(oteryn_client_runtime::chat_public_channels::ChatRoom),
    CreateOwn,
    Own(u16),
    Invitation {
        channel_id: u16,
        owner_name: String,
    },
    /// A client-local tab: Loot, Server Log or NPC.
    Local(super::ChannelTab),
}

/// The rows the server lets this session open: the public rooms of the channel list (or, while
/// no list arrived, the rooms already open), the own private channel and its invitations.
#[must_use]
pub fn channel_entries(
    list: Option<&oteryn_client_runtime::chat_private_channel::ChatChannelList>,
    open_rooms: oteryn_client_runtime::chat_public_channels::ChatRoomSet,
) -> Vec<ChannelEntry> {
    use oteryn_client_runtime::chat_public_channels::ChatRoom;
    let offered = list.map_or(open_rooms, |list| list.public_rooms);
    let mut entries: Vec<ChannelEntry> = ChatRoom::ALL
        .into_iter()
        .filter(|room| offered.contains(*room) || open_rooms.contains(*room))
        .map(ChannelEntry::Room)
        .collect();
    if let Some(list) = list {
        entries.push(
            list.own_channel
                .map_or(ChannelEntry::CreateOwn, ChannelEntry::Own),
        );
        entries.extend(list.invited.iter().map(|invite| ChannelEntry::Invitation {
            channel_id: invite.channel_id,
            owner_name: invite.owner_name.clone(),
        }));
    }
    entries
}

/// The rows of the channel list dialog: the server's channels, then the three local tabs.
#[must_use]
pub fn dialog_entries(
    list: Option<&oteryn_client_runtime::chat_private_channel::ChatChannelList>,
    open_rooms: oteryn_client_runtime::chat_public_channels::ChatRoomSet,
) -> Vec<ChannelEntry> {
    use super::ChannelTab;
    let mut entries = channel_entries(list, open_rooms);
    entries.extend(
        [ChannelTab::Loot, ChannelTab::ServerLog, ChannelTab::Npc]
            .into_iter()
            .map(ChannelEntry::Local),
    );
    entries
}

/// The channel list dialog text; a channel that already has a tab is marked open.
#[must_use]
pub fn channel_list_text(entries: &[ChannelEntry], console: &super::Console) -> String {
    let mut text = String::from(tr!("Channel list\n"));
    for entry in entries {
        text.push_str(&entry_text(entry, console));
    }
    text
}

/// One channel list row as a line of text.
pub(super) fn entry_text(entry: &ChannelEntry, console: &super::Console) -> String {
    use super::ChannelTab;
    let state = |tab: ChannelTab| {
        if console.channels.contains(&tab) {
            tr!("open")
        } else {
            tr!("closed")
        }
    };
    let mut text = String::new();
    match entry {
        ChannelEntry::Room(room) => {
            let _ = writeln!(
                text,
                "{}: {}",
                crate::chat_public_channels::room_name(*room),
                state(ChannelTab::Room(*room))
            );
        }
        ChannelEntry::CreateOwn => {
            text.push_str(tr!("Create"));
            text.push('\n');
        }
        ChannelEntry::Own(id) => {
            let _ = lwriteln!(text, "Own channel ({id})", id = id);
        }
        ChannelEntry::Invitation {
            channel_id,
            owner_name,
        } => {
            let _ = lwriteln!(text, "Invitation: {} ({})", owner_name, channel_id);
        }
        ChannelEntry::Local(tab) => {
            let _ = writeln!(text, "{}: {}", tab.label(), state(tab.clone()));
        }
    }
    text
}

/// Export exactly the selected console projection. The caller owns file mode and path.
pub fn write_log(panel: &ChatPanel, writer: &mut impl std::io::Write) -> std::io::Result<()> {
    writer.write_all(panel_text(panel).as_bytes())?;
    writer.flush()
}
