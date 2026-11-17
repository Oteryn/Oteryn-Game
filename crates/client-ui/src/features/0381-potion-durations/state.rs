use super::PotionDurations;
use std::time::Instant;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PotionDurationsPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub snapshot: Option<PotionDurations>,
    pub received_at: Option<Instant>,
    pub error: Option<&'static str>,
}
impl PotionDurationsPanel {
    pub fn finish(&mut self, accepted: bool, payload: &[u8]) -> bool {
        self.finish_at(accepted, payload, Instant::now())
    }
    pub fn finish_at(&mut self, accepted: bool, payload: &[u8], now: Instant) -> bool {
        self.pending = false;
        if accepted && let Some(snapshot) = PotionDurations::decode(payload) {
            self.snapshot = Some(snapshot);
            self.received_at = Some(now);
            self.error = None;
            return true;
        }
        self.error = Some(tr!("Potion durations could not be refreshed. Try Refresh."));
        false
    }
}
