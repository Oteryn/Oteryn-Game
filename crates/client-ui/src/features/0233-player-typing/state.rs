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
    pub indicators: Vec<TypingIndicator>,
    pub feedback: Option<&'static str>,
}
