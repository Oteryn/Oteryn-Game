use super::*;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Discovery {
    pub loot: Panel,
    pub inspection: crate::inspection::Panel,
    pub leader: crate::team_finder_leader::Panel,
    pub teams: crate::team_finder::Panel,
    pub history: crate::purchase_history::Panel,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub selected: bool,
    pub open: bool,
    pub pending: Option<Action>,
    pub feedback: String,
    pub item_feedback: Option<([u8; 16], &'static str)>,
    /// Confirmed edits made during this session.
    pub confirmed: Vec<Action>,
    /// Saved destinations, at most one per category.
    pub destinations: Vec<Action>,
    pub obtain_destinations: Vec<QuickLootDestination>,
    /// Live source displayed by the native Manage Loot Containers panel.
    pub distribution_source: Option<oteryn_client_runtime::inventory::ItemHandle>,
    pub distribution_result: Option<QuickLootResult>,
    pub managed: bool,
    pub destinations_selected: bool,
    pub filter_mode: Option<QuickLootFilterMode>,
    pub skipped: std::collections::BTreeSet<u32>,
    pub accepted: std::collections::BTreeSet<u32>,
    pub result: Option<QuickLootResult>,
}

impl Discovery {
    pub fn end(&mut self) {
        *self = Self::default();
        self.loot.feedback = tr!("Session ended").into();
        self.inspection.feedback = tr!("Session ended").into();
        self.leader.feedback = tr!("Session ended").into();
        self.teams.feedback = tr!("Session ended").into();
        self.history.feedback = tr!("Session ended").into();
    }
}
