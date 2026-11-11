#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Preview,
    Refresh,
    Projection,
    Set,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub artwork: Option<oteryn_client_runtime::cui3::house_podium::preview::Artwork>,
    pub graphic_worker: bool,
    pub preview_requested: bool,
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub pending_kind: u32,
    pub fields: [String; 4],
    pub feedback: String,
    pub summary: String,
    pub revision: Option<u64>,
    pub loaded_item: String,
    pub projection: String,
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
    /// Switch the draft between outfit, mount and creature displays and its facing.
    /// The committed projection is untouched; only the unsaved preview changes.
    pub fn choose_display(&mut self, kind: u8, direction: u8) -> bool {
        if !(1..=3).contains(&kind) || direction > 3 || self.pending {
            return false;
        }
        let (kind, direction) = (kind.to_string(), direction.to_string());
        if self.fields[1] != kind || self.fields[2] != direction {
            self.fields[1] = kind;
            self.fields[2] = direction;
            self.artwork = None;
        }
        true
    }
    pub fn failed(&mut self, message: &str) {
        self.pending = false;
        self.feedback = message.into();
    }
}
