//! Native controls for Depot Search.
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{apply_result, command, request};
pub use state::DepotSearchPanel;
pub use view::panel_text;
