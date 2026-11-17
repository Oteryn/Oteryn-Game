use super::*;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestPanel {
    pub state: QuestState,
    pub titles: std::collections::BTreeMap<u32, String>,
    pub open: bool,
    pub tracker_open: bool,
    pub mission: Option<u32>,
    pub list_search: String,
    pub list_sort: QuestSort,
    pub pending_query: Option<QuestLogQuery>,
    pub tracker: QuestTracker,
}

impl QuestPanel {
    pub fn title(&self, index: u32) -> String {
        self.titles
            .get(&index)
            .cloned()
            .unwrap_or_else(|| lfmt!("Quest {}", index))
    }

    pub fn set_list_options(&mut self, search: &str, sort: QuestSort) {
        if self.state.selected && search.len() <= 512 && !search.contains('\0') {
            self.list_search = search.to_owned();
            self.list_sort = sort;
        }
    }
}
