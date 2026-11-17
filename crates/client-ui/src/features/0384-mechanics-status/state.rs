use super::MechanicsStatus;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MechanicsStatusPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub snapshot: Option<MechanicsStatus>,
    pub error: Option<&'static str>,
}
impl MechanicsStatusPanel {
    pub fn finish(&mut self, accepted: bool, payload: &[u8]) -> bool {
        self.pending = false;
        if accepted && let Some(snapshot) = MechanicsStatus::decode(payload) {
            self.snapshot = Some(snapshot);
            self.error = None;
            return true;
        }
        self.error = Some(tr!(
            "Character effects could not be refreshed. Try Refresh."
        ));
        false
    }
}
