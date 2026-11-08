use oteryn_client_runtime::cyclopedia_character::CharacterTab;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CyclopediaCharacterPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub command_kind: u32,
    pub tab: CharacterTab,
    pub requested_tab: Option<CharacterTab>,
    pub offset: u16,
    pub limit: u8,
    pub target: Option<[u8; 16]>,
    pub result: Option<String>,
    pub store: Option<oteryn_client_runtime::cyclopedia_character::StoreSummary>,
    pub combat: Option<oteryn_client_runtime::cyclopedia_character::combat::CombatStats>,
    pub general: Option<oteryn_client_runtime::cyclopedia_character::GeneralStats>,
    pub history_tab: Option<CharacterTab>,
    pub history: Vec<HistoryRow>,
    pub kill_filter: Option<oteryn_client_runtime::cyclopedia_character::KillStatus>,
    pub feedback: Option<&'static str>,
}
impl Default for CyclopediaCharacterPanel {
    fn default() -> Self {
        Self {
            selected: false,
            open: false,
            pending: false,
            command_kind: 0,
            tab: CharacterTab::GeneralStats,
            requested_tab: None,
            offset: 0,
            limit: 20,
            target: None,
            result: None,
            store: None,
            combat: None,
            general: None,
            history_tab: None,
            history: Vec::new(),
            kill_filter: None,
            feedback: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRow {
    pub at: i64,
    pub status: Option<oteryn_client_runtime::cyclopedia_character::KillStatus>,
    pub description: String,
}
