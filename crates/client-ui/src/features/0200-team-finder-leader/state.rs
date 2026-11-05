use super::*;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub selected: bool,
    pub pending: bool,
    pub submitted: Option<Option<LeaderListing>>,
    pub confirmed: Option<LeaderListing>,
    pub feedback: String,
}
