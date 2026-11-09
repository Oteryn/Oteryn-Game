#[path = "view/atlas.rs"]
mod atlas;
#[path = "view/geometry.rs"]
pub mod atlas_view;
#[path = "state/atlas.rs"]
pub mod geometry;
pub use atlas::{atlas_action, atlas_color, atlas_detail};
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
