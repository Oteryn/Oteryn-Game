use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypingIndicator {
    pub identity: [u8; 16],
    pub generation: NonZeroU64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerTypingPanel {
    pub selected: bool,
    pub open: bool,
    pub own_typing: bool,
    pub pending: bool,
    pub(crate) composer_typing: bool,
    pub(crate) start_wanted: bool,
    pub(crate) last_start: Option<std::time::Instant>,
    pub indicators: Vec<TypingIndicator>,
    pub feedback: Option<&'static str>,
}

impl PlayerTypingPanel {
    /// Reset composer intent and its local rate gate on session initialization.
    pub fn initialize(&mut self, selected: bool) {
        self.selected = selected;
        self.composer_typing = false;
        self.start_wanted = false;
        self.last_start = None;
    }
}
