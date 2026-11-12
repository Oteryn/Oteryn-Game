//! Connection-local signed staff request state. Selection grants no staff authority.
#[derive(Debug, Clone, Default)]
pub struct Panel {
    pub selected: bool,
    pub tools: bool,
    pub open: bool,
    pub pending: bool,
    pub pending_kind: u32,
    pub fields: [String; 3],
    pub feedback: String,
    pub summary: String,
}
