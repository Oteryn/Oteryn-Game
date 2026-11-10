use super::ConditionStatus;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConditionStatusPanel {
    pub order: Vec<u8>,
    pub hidden: Vec<u8>,
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub snapshot: Option<ConditionStatus>,
    pub error: Option<&'static str>,
}
impl ConditionStatusPanel {
    pub fn finish(&mut self, accepted: bool, payload: &[u8]) -> bool {
        self.pending = false;
        if accepted && let Some(snapshot) = ConditionStatus::decode(payload) {
            self.snapshot = Some(snapshot);
            self.error = None;
            return true;
        }
        self.error = Some(tr!("Conditions could not be refreshed. Try Refresh."));
        false
    }
}
