use super::{DromeReply, Intent};
use oteryn_client_runtime::arena::{ArenaChallenge, ArenaResult};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArenaPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: Option<Intent>,
    pub challenges: Vec<ArenaChallenge>,
    pub result: Option<ArenaResult>,
    pub drome: Option<DromeReply>,
    pub hazards: oteryn_client_runtime::arena::DromeHazards,
    pub hazards_received_at: Option<std::time::Instant>,
    pub refreshed_at: Option<std::time::Instant>,
    pub feedback: Option<&'static str>,
}

impl ArenaPanel {
    pub fn end(&mut self) {
        self.selected = false;
        self.hazards = Default::default();
        self.hazards_received_at = None;
        if self.pending.take().is_some() {
            self.feedback = Some("The session has ended");
        }
    }
}
