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
/// A channel the player opened on top of the fixed console tabs, as in the Global client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelTab {
    Room(oteryn_client_runtime::chat_public_channels::ChatRoom),
    Private(u16),
    Conversation(String),
}
/// A channel tab to select once the server has confirmed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingFocus {
    Tab(ChannelTab),
    OwnChannel,
}
const FIXED_TABS: [ConsoleTab; 4] = [
    ConsoleTab::All,
    ConsoleTab::Local,
    ConsoleTab::Private,
    ConsoleTab::Guild,
];
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
    pub presentation: super::Presentation,
    pub communication: CommunicationLists,
    pub tab: ConsoleTab,
    pub mirror: Option<ConsoleTab>,
    pub mode: ComposerMode,
    pub show_messages: bool,
    pub filters: super::ConsoleFilters,
    pub notices: super::ConsoleNotices,
    tab_filters: [bool; 4],
    /// Opened channel tabs after the fixed tabs, in opening order.
    pub channels: Vec<ChannelTab>,
    /// The selected channel tab; `None` while a fixed tab is selected.
    pub current: Option<ChannelTab>,
    pub focus: Option<PendingFocus>,
}
impl Default for Console {
    fn default() -> Self {
        Self {
            display: Default::default(),
            metadata: Default::default(),
            presentation: Default::default(),
            communication: Default::default(),
            tab: ConsoleTab::All,
            mirror: None,
            mode: ComposerMode::On,
            show_messages: true,
            filters: Default::default(),
            notices: Default::default(),
            tab_filters: [true; 4],
            channels: Vec::new(),
            current: None,
            focus: None,
        }
    }
}
impl Console {
    pub fn select(&mut self, tab: ConsoleTab) {
        self.tab_filters[self.tab as usize] = self.show_messages;
        self.tab = tab;
        self.current = None;
        self.show_messages = self.tab_filters[tab as usize];
    }
    /// Open a channel tab (once) and select it.
    pub fn open_channel(&mut self, channel: ChannelTab) {
        if !self.channels.contains(&channel) {
            self.channels.push(channel.clone());
        }
        self.current = Some(channel);
    }
    /// Close the selected channel tab and fall back to its left neighbour. The fixed tabs,
    /// the default channel among them, cannot be closed.
    pub fn close_current(&mut self) -> Option<ChannelTab> {
        let closed = self.current.take()?;
        let at = self.channels.iter().position(|tab| *tab == closed)?;
        self.channels.remove(at);
        match at.checked_sub(1) {
            Some(left) => self.current = self.channels.get(left).cloned(),
            None => self.select(self.tab),
        }
        Some(closed)
    }
    /// Drop a channel tab that the server no longer offers, keeping the selection valid.
    pub fn drop_channel(&mut self, channel: &ChannelTab) {
        if self.current.as_ref() == Some(channel) {
            self.close_current();
        } else {
            self.channels.retain(|tab| tab != channel);
        }
    }
    /// Tab positions: the fixed tabs, then the opened channels.
    pub fn tab_count(&self) -> usize {
        FIXED_TABS.len() + self.channels.len()
    }
    pub fn tab_index(&self) -> usize {
        match &self.current {
            Some(current) => self
                .channels
                .iter()
                .position(|tab| tab == current)
                .map_or(0, |at| FIXED_TABS.len() + at),
            None => self.tab as usize,
        }
    }
    pub fn select_index(&mut self, index: usize) -> bool {
        if let Some(tab) = FIXED_TABS.get(index) {
            self.select(*tab);
        } else if let Some(channel) = self
            .channels
            .get(index.wrapping_sub(FIXED_TABS.len()))
            .cloned()
        {
            self.current = Some(channel);
        } else {
            return false;
        }
        true
    }
    /// Tab captions in tab order: the fixed tabs, then the opened channels.
    pub fn tab_labels(&self) -> Vec<String> {
        let mut labels = vec![
            tr!("All").to_owned(),
            tr!("Local").to_owned(),
            tr!("Private").to_owned(),
            tr!("Guild").to_owned(),
        ];
        labels.extend(self.channels.iter().map(|tab| match tab {
            ChannelTab::Room(room) => crate::chat_public_channels::room_name(*room).to_owned(),
            ChannelTab::Private(id) => lfmt!("Channel {id}", id = id),
            ChannelTab::Conversation(name) => name.clone(),
        }));
        labels
    }
    /// The server owns which public rooms are open: mirror them as tabs, selecting the room
    /// that was asked for once it opened.
    pub fn sync_rooms(&mut self, rooms: oteryn_client_runtime::chat_public_channels::ChatRoomSet) {
        use oteryn_client_runtime::chat_public_channels::ChatRoom;
        for room in ChatRoom::ALL {
            let tab = ChannelTab::Room(room);
            if !rooms.contains(room) {
                self.drop_channel(&tab);
            } else if !self.channels.contains(&tab) {
                self.channels.push(tab.clone());
            }
            if rooms.contains(room) && self.focus == Some(PendingFocus::Tab(tab.clone())) {
                self.focus = None;
                self.current = Some(tab);
            }
        }
    }
    /// Private-channel tabs follow the channel list: a channel that is neither owned nor an
    /// invitation is gone.
    pub fn sync_private(
        &mut self,
        list: Option<&oteryn_client_runtime::chat_private_channel::ChatChannelList>,
    ) {
        let gone: Vec<ChannelTab> = self
            .channels
            .iter()
            .filter(|tab| {
                matches!(tab, ChannelTab::Private(id) if !list.is_some_and(|list| {
                    list.own_channel == Some(*id)
                        || list.invited.iter().any(|invite| invite.channel_id == *id)
                }))
            })
            .cloned()
            .collect();
        for tab in &gone {
            self.drop_channel(tab);
        }
        if self.focus == Some(PendingFocus::OwnChannel)
            && let Some(id) = list.and_then(|list| list.own_channel)
        {
            self.focus = None;
            self.open_channel(ChannelTab::Private(id));
        }
    }
    /// Select the next (or previous) tab, wrapping around.
    pub fn cycle(&mut self, forward: bool) {
        let count = self.tab_count();
        let at = self.tab_index();
        self.select_index(if forward {
            (at + 1) % count
        } else {
            (at + count - 1) % count
        });
    }
    /// The rightmost third of the console is the mirror drop target.
    pub fn drop_tab(&mut self, tab: ConsoleTab, x: i32, width: i32) -> bool {
        if width <= 0 || x < width - width / 3 || x >= width {
            return false;
        }
        self.mirror = Some(tab);
        true
    }
    pub fn shows_messages(&self, tab: ConsoleTab) -> bool {
        if tab == self.tab {
            self.show_messages
        } else {
            self.tab_filters[tab as usize]
        }
    }
    pub fn close(&mut self) {
        self.mirror = None;
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
