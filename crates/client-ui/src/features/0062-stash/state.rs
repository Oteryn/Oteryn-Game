use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StashPanel {
    pub query: String,
    pub server_filtered: bool,
    pub category: String,
    pub trader: String,
    pub sort: Sort,
    pub reverse: bool,
    pub last_server_page: Option<u32>,
    pub sort_edit: u64,
    pub return_from_market: bool,
    pub selected: bool,
    pub actions_selected: bool,
    pub container_selected: bool,
    pub managed_selected: bool,
    pub container_batch: Batch,
    pub managed_batch: Batch,
    pub open: bool,
    pub page: u32,
    pub has_more: bool,
    pub rows: Vec<stash::StashRow>,
    pub row: Option<usize>,
    /// Live carried inventory and connected bag views, shared with the depot grid.
    pub carried: Vec<oteryn_client_runtime::depot::ItemEntry>,
    pub pending: bool,
    pub feedback: Option<String>,
    pub outcome: Option<stash::StashOutcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sort {
    #[default]
    Name,
    Stock,
}
