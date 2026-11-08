use super::*;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestPanel {
    pub state: QuestState,
    pub open: bool,
    pub mission: Option<u32>,
}
