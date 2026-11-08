//! Console preferences apply only to presentation of confirmed chat lines.
use oteryn_client_runtime::chat::{ChatLine, ChatSpeechMode};
use oteryn_client_runtime::chat_public_channels::ChatRoom;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConsoleTab {
    #[default]
    All,
    Local,
    Private,
    Guild,
    Party,
    Public(ChatRoom),
    Channel(u16),
}

impl ConsoleTab {
    pub fn label(self) -> String {
        match self {
            Self::Public(room) => crate::chat_public_channels::room_name(room).into(),
            Self::Channel(id) => format!("Channel {id}"),
            other => format!("{other:?}"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ComposerMode {
    #[default]
    On,
    Off,
    Temporary,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConsoleFilter {
    pub contains: String,
    pub hide_yells: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Console {
    pub tab: ConsoleTab,
    pub mode: ComposerMode,
    pub filters: BTreeMap<ConsoleTab, ConsoleFilter>,
}

impl Console {
    pub fn filter_mut(&mut self) -> &mut ConsoleFilter {
        self.filters.entry(self.tab).or_default()
    }

    pub fn tabs(&self, lines: &[ChatLine]) -> Vec<ConsoleTab> {
        let mut tabs = vec![
            ConsoleTab::All,
            ConsoleTab::Local,
            ConsoleTab::Private,
            ConsoleTab::Guild,
            ConsoleTab::Party,
        ];
        tabs.extend(ChatRoom::ALL.into_iter().map(ConsoleTab::Public));
        for line in lines {
            if let ChatLine::PrivateChannel(line) = line {
                let tab = ConsoleTab::Channel(line.channel_id);
                if !tabs.contains(&tab) {
                    tabs.push(tab);
                }
            }
        }
        tabs
    }

    pub fn accepts(&self, line: &ChatLine) -> bool {
        let tab = match line {
            ChatLine::Local { .. } => ConsoleTab::Local,
            ChatLine::Private { .. } => ConsoleTab::Private,
            ChatLine::Guild(_) => ConsoleTab::Guild,
            ChatLine::Party { .. } => ConsoleTab::Party,
            ChatLine::Room { room, .. } => ConsoleTab::Public(*room),
            ChatLine::PrivateChannel(line) => ConsoleTab::Channel(line.channel_id),
            ChatLine::Dropped => return true,
        };
        if self.tab != ConsoleTab::All && self.tab != tab {
            return false;
        }
        let Some(filter) = self.filters.get(&self.tab) else {
            return true;
        };
        if filter.hide_yells
            && matches!(
                line,
                ChatLine::Local {
                    mode: ChatSpeechMode::Yell,
                    ..
                }
            )
        {
            return false;
        }
        line_text(line)
            .to_lowercase()
            .contains(&filter.contains.to_lowercase())
    }
}

pub fn line_text(line: &ChatLine) -> String {
    match line {
        ChatLine::Local {
            speaker_name,
            mode,
            text,
            ..
        } => format!("[{mode:?}] {speaker_name}: {text}"),
        ChatLine::Private { speaker_name, text } => format!("[Private] {speaker_name}: {text}"),
        ChatLine::Room {
            room,
            speaker_name,
            text,
        } => format!(
            "[{}] {speaker_name}: {text}",
            crate::chat_public_channels::room_name(*room)
        ),
        ChatLine::Guild(line) => format!("[Guild] {}: {}", line.speaker_name, line.text),
        ChatLine::Party { speaker_name, text } => format!("[Party] {speaker_name}: {text}"),
        ChatLine::PrivateChannel(line) => format!(
            "[Channel {}] {}: {}",
            line.channel_id, line.speaker_name, line.text
        ),
        ChatLine::Dropped => "[Some chat lines were dropped]".into(),
    }
}
