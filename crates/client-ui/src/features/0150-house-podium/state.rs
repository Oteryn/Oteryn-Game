#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    Projection,
    Set,
    Preview,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub display_selected: bool,
    pub visibility: oteryn_client_runtime::house_podium_display::Visibility,
    pub preview: Option<oteryn_client_runtime::house_podium_display::State>,
    pub preview_open: bool,
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

impl Panel {
    /// No source-house target, revision or image may cross an activity scope boundary.
    pub fn reset_scope(&mut self) {
        *self = Self {
            selected: self.selected,
            display_selected: self.display_selected,
            ..Self::default()
        };
    }
}
