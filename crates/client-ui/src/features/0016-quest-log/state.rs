use super::*;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestPanel {
    pub state: QuestState,
    pub open: bool,
    pub tracker_open: bool,
    pub mission: Option<u32>,
    pub list_search: String,
    pub list_sort: QuestSort,
    pub pending_query: Option<QuestLogQuery>,
    pub tracker: QuestTracker,
}
