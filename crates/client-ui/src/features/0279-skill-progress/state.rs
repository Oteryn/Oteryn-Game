use super::SkillProgressSnapshot;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillProgressPanel {
    pub selected: bool,
    pub complete_selected: bool,
    pub complete: Option<oteryn_client_runtime::skill_display::SkillDisplay>,
    pub open: bool,
    pub pending: bool,
    pub snapshot: Option<SkillProgressSnapshot>,
    pub error: Option<&'static str>,
}

impl SkillProgressPanel {
    pub fn finish(&mut self, accepted: bool, payload: &[u8]) -> bool {
        self.pending = false;
        if self.complete_selected {
            if accepted && let Some(snapshot) = crate::skill_display::decode(payload) {
                self.snapshot = Some(snapshot.trained);
                self.complete = Some(snapshot);
                self.error = None;
                return true;
            }
            self.error = Some("Skill display could not be refreshed. Try Refresh.");
            return false;
        }
        if accepted && let Some(snapshot) = SkillProgressSnapshot::decode(payload) {
            self.snapshot = Some(snapshot);
            self.error = None;
            return true;
        }
        self.error = Some("Skill progress could not be refreshed. Try Refresh.");
        false
    }
}
