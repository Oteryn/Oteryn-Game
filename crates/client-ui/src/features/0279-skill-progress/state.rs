use super::SkillProgressSnapshot;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillProgressPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub snapshot: Option<SkillProgressSnapshot>,
    pub error: Option<&'static str>,
}

impl SkillProgressPanel {
    pub fn finish(&mut self, accepted: bool, payload: &[u8]) -> bool {
        self.pending = false;
        if accepted && let Some(snapshot) = SkillProgressSnapshot::decode(payload) {
            self.snapshot = Some(snapshot);
            self.error = None;
            return true;
        }
        self.error = Some("Skill progress could not be refreshed. Try Refresh.");
        false
    }
}
