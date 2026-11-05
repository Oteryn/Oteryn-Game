use super::*;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub last_request: Option<Request>,
    pub reply: Option<Reply>,
    pub feedback: String,
}
