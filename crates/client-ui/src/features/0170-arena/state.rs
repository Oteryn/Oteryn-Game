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
    pub feedback: Option<&'static str>,
}

impl ArenaPanel {
    pub fn end(&mut self) {
        self.selected = false;
        if self.pending.take().is_some() {
            self.feedback = Some(tr!("The session has ended"));
        }
    }
}
