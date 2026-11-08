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
    pub tab: ConsoleTab,
    pub mode: ComposerMode,
    pub show_messages: bool,
    filters: [bool; 4],
}
impl Default for Console {
    fn default() -> Self {
        Self {
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
