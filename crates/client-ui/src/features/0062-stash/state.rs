use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StashPanel {
    pub query: String,
    pub server_filtered: bool,
    pub category: String,
    pub trader: String,
    pub sort: Sort,
    pub sort_edit: u64,
    pub return_from_market: bool,
    pub selected: bool,
    pub open: bool,
    pub page: u32,
    pub has_more: bool,
    pub rows: Vec<stash::StashRow>,
    pub row: Option<usize>,
    /// Live carried inventory and connected bag views, shared with the depot grid.
    pub carried: Vec<oteryn_client_runtime::depot::ItemEntry>,
    pub pending: bool,
    pub feedback: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sort {
    #[default]
    Name,
    Stock,
}
