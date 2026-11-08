#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Query,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub pending_kind: u32,
    pub fields: [String; 0],
    pub feedback: String,
    pub summary: String,
    pub skull_timer: Option<SkullTimer>,
    pub icons: Vec<oteryn_client_runtime::party_skull::PartyIcon>,
}
impl Panel {
    pub fn begin(&mut self, kind: u32) -> bool {
        if !self.selected {
            self.feedback = "Unavailable in this session".into();
            return false;
        }
        if self.pending {
            self.feedback = "Waiting for the server".into();
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

/// Remaining duration is supplied by the server; a skull icon alone has no implied expiry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkullTimer {
    pub skull: u8,
    pub remaining_ms: Option<u64>,
}
impl SkullTimer {
    pub fn remaining_seconds(self, elapsed_ms: u64) -> Option<u64> {
        self.remaining_ms
            .map(|remaining| remaining.saturating_sub(elapsed_ms).div_ceil(1000))
    }
    pub fn text(self) -> String {
        let name = match self.skull {
            0 => "No",
            1 => "Yellow",
            2 => "Green",
            3 => "White",
            4 => "Red",
            5 => "Black",
            6 => "Orange",
            _ => "Unknown",
        };
        match self.remaining_seconds(0) {
            Some(seconds) => format!("{name} skull: {seconds}s at observation"),
            None => format!("{name} skull: expiry unavailable"),
        }
    }
}
