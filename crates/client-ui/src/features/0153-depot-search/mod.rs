//! Native controls for Depot Search.
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{apply_result, command, detail, request, search};
pub use state::DepotSearchPanel;
pub use view::panel_text;
