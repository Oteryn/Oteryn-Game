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
    /// What each tracker window (Bestiary, then Bosstiary) last heard about its own requests.
    pub widget_feedback: [Option<&'static str>; 2],
    /// Per tracker window, the request waiting for the other window's request in flight.
    pub queued: [Option<(super::Intent, String)>; 2],
}

impl CyclopediaTrackerPanel {
    /// Whether the Bestiary (`boss` false) or Bosstiary tracker has a request in flight or
    /// waiting; the other tracker stays usable.
    #[must_use]
    pub fn tracker_busy(&self, boss: bool) -> bool {
        (self.pending && self.boss == boss) || self.queued[usize::from(boss)].is_some()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TrackerSort {
    #[default]
    Name,
    Kills,
    Remaining,
}
