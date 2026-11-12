use super::wire;
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Panel {
    pub selected: bool,
    pub open: bool,
    pub entries: Vec<wire::Entry>,
    pub identity: Option<[u8; 16]>,
    pub pending: Option<wire::Request>,
    pub feedback: Option<&'static str>,
}
#[derive(Debug, Clone, Copy)]
pub enum Intent {
    List,
    Return,
    Outfit { look_type: u16, colors: [u8; 4] },
}
