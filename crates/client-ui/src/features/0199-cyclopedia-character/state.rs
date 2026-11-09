use oteryn_client_runtime::cyclopedia_character::CharacterTab;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CyclopediaCharacterPanel {
    pub protection: super::Protection,
    pub items: super::ItemSummary,
    pub gallery: super::Gallery,
    pub badges: Option<Vec<u8>>,
    pub badge_filter: BadgeFilter,
    pub hidden_badges: std::collections::BTreeSet<u8>,
    pub selected: bool,
    pub blessings: super::protocol::BlessingsPanel,
    pub open: bool,
    pub pending: bool,
    pub command_kind: u32,
    pub tab: CharacterTab,
    pub requested_tab: Option<CharacterTab>,
    pub misc_resources_due: bool,
    pub auxiliary_pending: bool,
    pub misc_general: Option<oteryn_client_runtime::cyclopedia_character::GeneralStats>,
    pub achievements: crate::account_achievements::AchievementsPanel,
    pub achievement_profile: Option<([u8; 16], String)>,
    pub achievement_pending: bool,
    pub profile_generation: u64,
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
            protection: Default::default(),
            items: Default::default(),
            gallery: Default::default(),
            badges: None,
            badge_filter: Default::default(),
            hidden_badges: Default::default(),
            selected: false,
            blessings: Default::default(),
            open: false,
            pending: false,
            command_kind: 0,
            tab: CharacterTab::GeneralStats,
            requested_tab: None,
            misc_resources_due: false,
            auxiliary_pending: false,
            misc_general: None,
            achievements: Default::default(),
            achievement_profile: None,
            achievement_pending: false,
            profile_generation: 0,
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

impl CyclopediaCharacterPanel {
    /// A new profile must never inherit the previously displayed profile's facts.
    pub fn change_target(&mut self, target: Option<[u8; 16]>) {
        if self.target != target {
            self.target = target;
            self.protection = Default::default();
            self.items = Default::default();
            self.gallery = Default::default();
            self.badges = None;
            self.hidden_badges.clear();
            self.profile_generation = self.profile_generation.saturating_add(1);
            self.achievements = Default::default();
            self.achievement_profile = None;
            self.achievement_pending = false;
            self.result = None;
            self.general = None;
            self.misc_general = None;
            self.combat = None;
            self.store = None;
            self.history.clear();
            self.history_tab = None;
            self.misc_resources_due = false;
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BadgeFilter {
    #[default]
    All,
    Earned,
    Locked,
    Hidden,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BadgeRow {
    pub id: u8,
    pub name: &'static str,
    pub earned: bool,
    pub hidden: bool,
}
