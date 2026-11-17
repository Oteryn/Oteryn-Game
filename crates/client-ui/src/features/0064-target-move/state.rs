use super::WalkResponse;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WalkState {
    pub selected: bool,
    pub pending: bool,
    pub response: Option<WalkResponse>,
    pub feedback: Option<&'static str>,
}
