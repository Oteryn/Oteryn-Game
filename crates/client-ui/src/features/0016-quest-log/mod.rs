//! Quest list, mission selection and authoritative tracking controls.
pub use oteryn_client_runtime::quest_log::*;
mod input;
#[path = "view/list.rs"]
mod list;
mod protocol;
mod state;
mod view;
pub use input::{open_key, tracker_key, tracking_query};
pub use list::{QUEST_SORT_LABELS, QuestSort, quest_row_query, quest_rows};
pub use protocol::request;
pub use state::QuestPanel;
pub use view::{panel_text, tracker_text};
#[cfg(test)]
#[path = "tests/list_options.rs"]
mod list_options_tests;

#[cfg(test)]
#[path = "tests/view.rs"]
mod tests;

#[path = "state/tracker.rs"]
mod tracker;
pub use tracker::{QuestTracker, TrackerFilter, sync_tracker_search, tracker_lines, tracker_query};

#[cfg(test)]
#[path = "tests/tracker.rs"]
mod tracker_tests;
