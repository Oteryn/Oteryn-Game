pub use crate::chat_public_channels::ChatPanel;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Say,
    Whisper,
    Yell,
    Private,
    Guild,
}

// Local console preferences; changing tabs never discards received messages.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ConsoleTab {
    #[default]
    All,
    Local,
    Private,
    Guild,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ComposerMode {
    #[default]
    On,
    Off,
    Temporary,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Console {
    pub display: super::ConsoleDisplay,
    pub metadata: super::Metadata,
    pub communication: CommunicationLists,
    pub tab: ConsoleTab,
    pub mode: ComposerMode,
    pub show_messages: bool,
    filters: [bool; 4],
}
impl Default for Console {
    fn default() -> Self {
        Self {
            display: Default::default(),
            metadata: Default::default(),
            communication: Default::default(),
            tab: ConsoleTab::All,
            mode: ComposerMode::On,
            show_messages: true,
            filters: [true; 4],
        }
    }
}
impl Console {
    pub fn select(&mut self, tab: ConsoleTab) {
        self.filters[self.tab as usize] = self.show_messages;
        self.tab = tab;
        self.show_messages = self.filters[tab as usize];
    }
    pub fn temporary_on(&mut self) {
        self.mode = ComposerMode::Temporary;
    }
    pub fn can_send(&self) -> bool {
        self.mode != ComposerMode::Off
    }
    pub fn sent(&mut self) {
        if self.mode == ComposerMode::Temporary {
            self.mode = ComposerMode::Off;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommunicationList {
    Ignore,
    Whitelist,
    Blacklist,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommunicationLists {
    pub ignored: std::collections::BTreeSet<String>,
    pub whitelist: std::collections::BTreeSet<String>,
    pub blacklist: std::collections::BTreeSet<String>,
}
impl CommunicationLists {
    pub fn edit(&mut self, list: CommunicationList, name: &str, add: bool) -> bool {
        let name = name.trim();
        if name.is_empty() || name.len() > 120 || name.contains('\0') {
            return false;
        }
        let name = name.to_lowercase();
        let set = match list {
            CommunicationList::Ignore => &mut self.ignored,
            CommunicationList::Whitelist => &mut self.whitelist,
            CommunicationList::Blacklist => &mut self.blacklist,
        };
        if add {
            if set.len() >= 200 && !set.contains(&name) {
                return false;
            }
            set.insert(name);
        } else {
            set.remove(&name);
        }
        true
    }
    pub fn allows(&self, name: &str) -> bool {
        let name = name.trim().to_lowercase();
        self.whitelist.contains(&name)
            || (!self.ignored.contains(&name) && !self.blacklist.contains(&name))
    }
    pub fn visible(&self, line: &oteryn_client_runtime::chat::ChatLine) -> bool {
        use oteryn_client_runtime::chat::ChatLine;
        match line {
            ChatLine::Local { speaker_name, .. }
            | ChatLine::Private { speaker_name, .. }
            | ChatLine::Room { speaker_name, .. }
            | ChatLine::Party { speaker_name, .. } => self.allows(speaker_name),
            ChatLine::Guild(line) => self.allows(&line.speaker_name),
            ChatLine::PrivateChannel(line) => self.allows(&line.speaker_name),
            ChatLine::Dropped => true,
        }
    }
}
