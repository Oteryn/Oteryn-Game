use oteryn_client_runtime::arena::{ArenaChallenge, ArenaResult};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArenaPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: Option<u32>,
    pub challenges: Vec<ArenaChallenge>,
    pub result: Option<ArenaResult>,
    pub feedback: Option<&'static str>,
}
