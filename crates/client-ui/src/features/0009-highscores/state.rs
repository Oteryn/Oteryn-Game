use oteryn_client_runtime::highscores::wire::{HighscoresQuery, HighscoresResult};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighscoresPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub category: u32,
    pub vocation: u32,
    pub reply: Option<HighscoresResult>,
    pub feedback: Option<String>,
    pub(super) query: Option<HighscoresQuery>,
}
impl Default for HighscoresPanel {
    fn default() -> Self {
        Self {
            selected: false,
            open: false,
            pending: false,
            category: 1,
            vocation: 0,
            reply: None,
            feedback: None,
            query: None,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum Action {
    Refresh,
    ShowMe,
    Next,
    Previous,
    Filter { category: u32, vocation: u32 },
}
