use oteryn_client_runtime::boss_encounter::Snapshot;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BossEncounterPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub pending_kind: u32,
    pub fields: [String; 3],
    pub summary: String,
    pub rewards: Option<oteryn_client_runtime::boss_encounter::RewardChest>,
    pub snapshot: Option<Snapshot>,
    pub feedback: Option<&'static str>,
}
