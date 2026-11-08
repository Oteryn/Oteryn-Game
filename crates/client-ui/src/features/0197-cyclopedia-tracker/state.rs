use oteryn_client_runtime::cyclopedia_tracker::TrackerSnapshot;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CyclopediaTrackerPanel {
    pub selected: bool,
    pub open: bool,
    pub bestiary_widget_open: bool,
    pub bosstiary_widget_open: bool,
    pub bosstiary_snapshot: Option<TrackerSnapshot>,
    pub boss_sort: TrackerSort,
    pub boss_hide_completed: bool,
    pub boss_search: String,
    pub bestiary_snapshot: Option<TrackerSnapshot>,
    pub sort: TrackerSort,
    pub hide_completed: bool,
    pub search: String,
    pub pending: bool,
    pub boss: bool,
    pub creature_key: String,
    pub snapshot: Option<TrackerSnapshot>,
    pub feedback: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TrackerSort {
    #[default]
    Name,
    Kills,
    Remaining,
}
