#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Enter,
    Exit,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub pending_kind: u32,
    pub fields: [String; 1],
    pub feedback: String,
    pub summary: String,
}
impl Panel {
    pub fn begin(&mut self, kind: u32) -> bool {
        if !self.selected {
            self.feedback = tr!("Unavailable in this session").into();
            return false;
        }
        if self.pending {
            self.feedback = tr!("Waiting for the server").into();
            return false;
        }
        self.pending = true;
        self.pending_kind = kind;
        self.feedback.clear();
        true
    }
    pub fn failed(&mut self, message: &str) {
        self.pending = false;
        self.feedback = message.into();
    }
}
