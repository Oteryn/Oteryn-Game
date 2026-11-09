#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Query,
    Atlas,
    RegionAtlas,
    AtlasNext,
    Start,
    Shuffle,
    Find,
    Donate,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub geometry: super::geometry::Atlas,
    pub atlas: Option<oteryn_client_runtime::cyclopedia_map::atlas::Atlas>,
    pub atlas_query: oteryn_client_runtime::cyclopedia_map::atlas::Query,
    pub atlas_open: bool,
    pub atlas_layers: u8,
    pub atlas_cell: usize,
    pub section: Section,
    pub map: Option<oteryn_client_runtime::cyclopedia_map::MapResult>,
    pub region_atlas: Option<oteryn_client_runtime::cyclopedia_map::region_atlas::Page>,
    pub atlas_after: u32,
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
    pub fn title(self) -> &'static str {
        match self {
            Self::Character => tr!("Character"),
            Self::Items => tr!("Items"),
            Self::Map => tr!("Map"),
            Self::Bestiary => tr!("Bestiary"),
            Self::Charms => tr!("Charms"),
            Self::Bosses => tr!("Bosses"),
            Self::Houses => tr!("Houses"),
            Self::Analytics => tr!("Analytics"),
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
            .chain(
                self.region_atlas
                    .iter()
                    .flat_map(|a| &a.runs)
                    .map(|r| r.floor),
            )
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
        self.geometry = Default::default();
        self.region_atlas = None;
        self.atlas = None;
        self.map = None;
        self.summary.clear();
    }
}
