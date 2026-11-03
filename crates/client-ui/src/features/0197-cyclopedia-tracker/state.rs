use oteryn_client_runtime::cyclopedia_tracker::TrackerSnapshot;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CyclopediaTrackerPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub boss: bool,
    pub creature_key: String,
    pub snapshot: Option<TrackerSnapshot>,
    pub feedback: Option<&'static str>,
}
