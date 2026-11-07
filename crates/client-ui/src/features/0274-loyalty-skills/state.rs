use super::LoyaltySkills;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoyaltySkillsPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub snapshot: Option<LoyaltySkills>,
    pub error: Option<&'static str>,
}

impl LoyaltySkillsPanel {
    pub fn finish(&mut self, accepted: bool, payload: &[u8]) -> bool {
        self.pending = false;
        if accepted && let Some(snapshot) = LoyaltySkills::decode(payload) {
            self.snapshot = Some(snapshot);
            self.error = None;
            return true;
        }
        self.error = Some("Skills could not be refreshed. Try Refresh.");
        false
    }
}
