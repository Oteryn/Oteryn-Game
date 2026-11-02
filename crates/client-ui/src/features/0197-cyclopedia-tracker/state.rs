#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CyclopediaTrackerPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub last_command: Option<u32>,
    pub result: Vec<u8>,
    pub feedback: Option<&'static str>,
}
