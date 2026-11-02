use oteryn_client_runtime::boss_difficulty::Reply;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BossDifficultyPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub boss_key: String,
    pub difficulty: u16,
    pub context: Option<[u8; 16]>,
    pub reply: Option<Reply>,
    pub feedback: Option<&'static str>,
}
