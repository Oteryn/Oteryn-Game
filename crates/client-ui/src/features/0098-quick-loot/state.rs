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
    /// Only confirmed edits from this session; the existing wire has no settings query.
    pub confirmed: Vec<Action>,
    /// Current destinations confirmed in this session, at most one per category.
    pub destinations: Vec<Action>,
    pub managed: bool,
    pub skipped: std::collections::BTreeSet<u32>,
    pub accepted: std::collections::BTreeSet<u32>,
    pub result: Option<QuickLootResult>,
}

impl Discovery {
    pub fn end(&mut self) {
        *self = Self::default();
        self.loot.feedback = "Session ended".into();
        self.inspection.feedback = "Session ended".into();
        self.leader.feedback = "Session ended".into();
        self.teams.feedback = "Session ended".into();
        self.history.feedback = "Session ended".into();
    }
}
