use oteryn_client_runtime::boss_encounter::Snapshot;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BossEncounterPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub snapshot: Option<Snapshot>,
    pub feedback: Option<&'static str>,
}
