#[path = "view/map.rs"]
mod map;
mod protocol;
mod state;
mod view;
pub use map::{MapHouse, map_houses};
pub use protocol::request;
pub use state::Panel;
pub use view::panel_text;
