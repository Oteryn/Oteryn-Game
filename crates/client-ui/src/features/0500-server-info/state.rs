use super::Query;
#[derive(Debug, Clone, Default)]
pub struct Panel {
    pub selected: bool,
    pub pending: Vec<Query>,
}
