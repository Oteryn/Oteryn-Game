use oteryn_client_runtime::boss_difficulty::{PityTracker, Reply};
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BossDifficultyPanel {
    pub selected: bool,
    /// Capability 492 was selected: windows of pity bosses carry the tracker.
    pub pity_selected: bool,
    pub pity: Option<PityTracker>,
    pub open: bool,
    pub pending: bool,
    pub boss_key: String,
    pub difficulty: u16,
    pub context: Option<[u8; 16]>,
    pub reply: Option<Reply>,
    pub feedback: Option<&'static str>,
}
