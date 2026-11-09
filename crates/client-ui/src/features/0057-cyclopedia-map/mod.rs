#[path = "state/atlas.rs"]
pub mod atlas;
#[path = "view/atlas.rs"]
pub mod atlas_view;
mod input;
mod state;
mod view;
pub use input::open_key;
pub use state::{Intent, Panel, Section};
pub use view::{MapPoint, map_page, map_points, map_text, panel_text};
mod protocol;
pub use protocol::request;

#[cfg(test)]
mod tests;

#[path = "view/region_atlas.rs"]
mod region_atlas_view;
pub use region_atlas_view::{AtlasRun, atlas_pois, atlas_runs};
