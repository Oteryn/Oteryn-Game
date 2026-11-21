use oteryn_client_runtime::highscores::wire::{HighscoresQuery, HighscoresResult};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighscoresPanel {
    pub selected: bool,
    /// Capability 498: category 13, Loyalty, is offered.
    pub loyalty_selected: bool,
    pub catalogue_selected: bool,
    pub catalogue_loaded: bool,
    pub catalogue_pending: bool,
    pub catalogue: Vec<oteryn_client_runtime::highscores::catalogue::Entry>,
    pub world_type: Option<oteryn_client_runtime::highscores::catalogue::WorldType>,
    pub open: bool,
    pub pending: bool,
    pub world: Option<oteryn_client_runtime::highscores::wire::WorldId>,
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
            loyalty_selected: false,
            catalogue_selected: false,
            catalogue_loaded: false,
            catalogue_pending: false,
            catalogue: Vec::new(),
            world_type: None,
            open: false,
            pending: false,
            world: None,
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
    Catalogue,
    WorldType(Option<oteryn_client_runtime::highscores::catalogue::WorldType>),
    Refresh,
    World(Option<oteryn_client_runtime::highscores::wire::WorldId>),
    ShowMe,
    Next,
    Previous,
    Filter { category: u32, vocation: u32 },
}
