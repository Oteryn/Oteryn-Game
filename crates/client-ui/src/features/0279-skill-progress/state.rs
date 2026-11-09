use super::SkillProgressSnapshot;
pub const SKILL_LABELS: [&str; 8] = [
    "Magic",
    "Fist",
    "Club",
    "Sword",
    "Axe",
    "Distance",
    "Shielding",
    "Fishing",
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillProgressPanel {
    pub hidden_skills: u8,
    pub selected: bool,
    pub complete_selected: bool,
    pub complete: Option<oteryn_client_runtime::skill_display::SkillDisplay>,
    pub open: bool,
    pub pending: bool,
    pub snapshot: Option<SkillProgressSnapshot>,
    pub error: Option<&'static str>,
}

impl SkillProgressPanel {
    pub fn toggle_skill(&mut self, index: usize) {
        if index < 8 {
            self.hidden_skills ^= 1 << index;
        }
    }
    #[must_use]
    pub fn gauges(&self) -> Vec<(&'static str, u16, bool)> {
        self.snapshot.map_or_else(Vec::new, |snapshot| {
            SKILL_LABELS
                .into_iter()
                .zip(snapshot.skills)
                .enumerate()
                .map(|(index, (label, skill))| {
                    (
                        crate::localization::tr(label),
                        skill.percent.min(10000),
                        self.hidden_skills & (1 << index) == 0,
                    )
                })
                .collect()
        })
    }

    pub fn finish(&mut self, accepted: bool, payload: &[u8]) -> bool {
        self.pending = false;
        if self.complete_selected {
            if accepted && let Some(snapshot) = crate::skill_display::decode(payload) {
                self.snapshot = Some(snapshot.trained);
                self.complete = Some(snapshot);
                self.error = None;
                return true;
            }
            self.error = Some(tr!("Skill display could not be refreshed. Try Refresh."));
            return false;
        }
        if accepted && let Some(snapshot) = SkillProgressSnapshot::decode(payload) {
            self.snapshot = Some(snapshot);
            self.error = None;
            return true;
        }
        self.error = Some(tr!("Skill progress could not be refreshed. Try Refresh."));
        false
    }
}
