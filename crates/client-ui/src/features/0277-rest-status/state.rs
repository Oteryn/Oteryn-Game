use super::RestStatus;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RestStatusPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub snapshot: Option<RestStatus>,
    pub error: Option<&'static str>,
}

impl RestStatusPanel {
    pub fn finish(&mut self, accepted: bool, payload: &[u8]) -> bool {
        self.pending = false;
        if self.selected
            && accepted
            && let Some(snapshot) = RestStatus::decode(payload)
        {
            self.snapshot = Some(snapshot);
            self.error = None;
            return true;
        }
        // A refused refresh cannot keep presenting old area/food bonuses as current.
        self.snapshot = None;
        self.error = Some(tr!("Rest status could not be refreshed. Try Refresh."));
        false
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
