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
    pub atlas: super::atlas::Atlas,
    pub section: Section,
    pub map: Option<oteryn_client_runtime::cyclopedia_map::MapResult>,
    pub floor: i8,
    pub hide_pois: bool,
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

impl Panel {
    /// Only floors present in the confirmed exploration result can be selected.
    pub fn select_floor(&mut self, previous: bool) {
        let floors: Vec<_> = self
            .map
            .iter()
            .flat_map(|map| &map.visible)
            .map(|point| point.2)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        if floors.is_empty() {
            return;
        }
        let index = floors
            .iter()
            .position(|floor| *floor == self.floor)
            .map_or(0, |index| {
                if previous {
                    (index + floors.len() - 1) % floors.len()
                } else {
                    (index + 1) % floors.len()
                }
            });
        self.floor = floors[index];
    }
}

impl Panel {
    pub fn invalidate_atlas(&mut self) {
        self.atlas = Default::default();
        self.map = None;
        self.summary.clear();
    }
}
