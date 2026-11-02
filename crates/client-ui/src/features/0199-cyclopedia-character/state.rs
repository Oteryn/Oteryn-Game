use oteryn_client_runtime::cyclopedia_character::CharacterTab;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CyclopediaCharacterPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub command_kind: u32,
    pub tab: CharacterTab,
    pub offset: u16,
    pub limit: u8,
    pub target: Option<[u8; 16]>,
    pub result: Option<String>,
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
            offset: 0,
            limit: 20,
            target: None,
            result: None,
            feedback: None,
        }
    }
}
