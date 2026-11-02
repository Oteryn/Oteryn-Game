#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BrowseFieldPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub x: i32,
    pub y: i32,
    pub floor: i16,
    pub view_id: u8,
    pub index: u16,
    pub response: Vec<u8>,
    pub feedback: Option<String>,
}
