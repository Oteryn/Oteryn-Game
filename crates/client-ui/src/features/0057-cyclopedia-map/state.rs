#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Query,
    Start,
    Shuffle,
    Find,
    Donate,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub section: Section,
    pub map: Option<oteryn_client_runtime::cyclopedia_map::MapResult>,
    pub floor: i8,
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

/// Sections share one Cyclopedia host and retain their own confirmed state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Section {
    Character,
    Items,
    #[default]
    Map,
    Bestiary,
    Charms,
    Bosses,
    Houses,
    Analytics,
}
impl Section {
    pub const ALL: [Self; 8] = [
        Self::Character,
        Self::Items,
        Self::Map,
        Self::Bestiary,
        Self::Charms,
        Self::Bosses,
        Self::Houses,
        Self::Analytics,
    ];
    pub const fn title(self) -> &'static str {
        match self {
            Self::Character => "Character",
            Self::Items => "Items",
            Self::Map => "Map",
            Self::Bestiary => "Bestiary",
            Self::Charms => "Charms",
            Self::Bosses => "Bosses",
            Self::Houses => "Houses",
            Self::Analytics => "Analytics",
        }
    }
}
impl Panel {
    pub fn navigate(&mut self, section: Section) {
        self.section = section;
    }
}
