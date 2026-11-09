use super::PartyEffects;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartyEffectsPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub snapshot: Option<PartyEffects>,
    pub error: Option<&'static str>,
}
impl PartyEffectsPanel {
    pub fn finish(&mut self, accepted: bool, payload: &[u8]) -> bool {
        self.pending = false;
        if accepted && let Some(value) = PartyEffects::decode(payload) {
            self.snapshot = Some(value);
            self.error = None;
            return true;
        }
        self.error = Some(tr!("Party effects could not be refreshed. Try Refresh."));
        false
    }
}
