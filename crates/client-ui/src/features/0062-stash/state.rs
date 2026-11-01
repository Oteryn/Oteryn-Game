use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StashPanel {
    pub selected: bool,
    pub open: bool,
    pub page: u32,
    pub has_more: bool,
    pub rows: Vec<stash::StashRow>,
    pub row: Option<usize>,
    pub pending: bool,
    pub feedback: Option<String>,
}
