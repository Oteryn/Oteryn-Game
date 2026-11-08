use oteryn_client_runtime::boss_encounter::Snapshot;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BossEncounterPanel {
    pub selected: bool,
    pub open: bool,
    pub cooldowns_open: bool,
    pub cooldown_received_at: Option<std::time::Instant>,
    pub pending: bool,
    pub pending_kind: u32,
    pub fields: [String; 3],
    pub browser_search: String,
    pub browser_category: super::BossCategory,
    pub summary: String,
    pub hazard: Option<(u8, u8)>,
    pub rewards: Option<oteryn_client_runtime::boss_encounter::RewardChest>,
    pub snapshot: Option<Snapshot>,
    pub feedback: Option<&'static str>,
}
