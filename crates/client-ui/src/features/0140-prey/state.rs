use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreyPanel {
    pub selected: bool,
    pub open: bool,
    pub reply: Option<prey::Reply>,
    pub slot: u8,
    pub candidate: u8,
    pub creature: String,
    pub option: u8,
    pub pending: bool,
    pub feedback: Option<&'static str>,
}

impl Default for PreyPanel {
    fn default() -> Self {
        Self {
            selected: false,
            open: false,
            reply: None,
            slot: 1,
            candidate: 1,
            creature: String::new(),
            option: 0,
            pending: false,
            feedback: None,
        }
    }
}
