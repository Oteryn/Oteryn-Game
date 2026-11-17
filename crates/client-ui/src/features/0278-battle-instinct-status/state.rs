use super::BattleInstinctStatus;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BattleInstinctStatusPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub snapshot: Option<BattleInstinctStatus>,
    pub error: Option<&'static str>,
}

impl BattleInstinctStatusPanel {
    pub fn finish(&mut self, accepted: bool, payload: &[u8]) -> bool {
        self.pending = false;
        if accepted && let Some(snapshot) = BattleInstinctStatus::decode(payload) {
            self.snapshot = Some(snapshot);
            self.error = None;
            return true;
        }
        self.error = Some(tr!("Battle Instinct could not be refreshed. Try Refresh."));
        false
    }
}
