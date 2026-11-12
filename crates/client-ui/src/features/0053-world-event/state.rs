use oteryn_client_runtime::world_event::WorldEventState;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldEventPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub state: Option<WorldEventState>,
    pub feedback: Option<&'static str>,
}
