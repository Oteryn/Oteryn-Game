use oteryn_client_runtime::cyclopedia_map::atlas::{Area, Request, Run};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atlas {
    pub open: bool,
    pub pending: Option<Request>,
    pub parent: u32,
    pub after: u16,
    pub next_areas: Option<u16>,
    pub areas: Vec<Area>,
    pub selected: Option<u32>,
    pub floor: i8,
    pub bounds: Option<[u16; 4]>,
    pub total: u32,
    pub runs: Vec<Run>,
    pub next_geometry: Option<u16>,
    pub auto_load: bool,
    pub hide_geometry: bool,
    pub revision: u64,
    pub feedback: String,
    pub lifetime: std::sync::Arc<()>,
}
impl Default for Atlas {
    fn default() -> Self {
        Self {
            open: false,
            pending: None,
            parent: 0,
            after: 0,
            next_areas: None,
            areas: Vec::new(),
            selected: None,
            floor: -7,
            bounds: None,
            total: 0,
            runs: Vec::new(),
            next_geometry: None,
            auto_load: false,
            hide_geometry: false,
            revision: 0,
            feedback: String::new(),
            lifetime: std::sync::Arc::new(()),
        }
    }
}
impl Atlas {
    pub fn area(&self) -> Option<&Area> {
        self.areas.iter().find(|a| Some(a.id) == self.selected)
    }
    pub fn clear_geometry(&mut self) {
        self.feedback = "Load an area to view its footprint".into();
        self.runs.clear();
        self.bounds = None;
        self.total = 0;
        self.next_geometry = None;
        self.auto_load = false;
        self.revision = self.revision.wrapping_add(1);
    }
}
