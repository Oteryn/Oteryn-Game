//! Server-confirmed market offers and controls.
pub use oteryn_client_runtime::market::{CommandStatus, MarketCommand, MarketResponse, market};
#[path = "state/details.rs"]
pub mod details;
mod input;
mod protocol;
mod state;
mod view;
pub use input::{edit_details, open_key};
pub use protocol::{Intent, apply_result, request};
pub use state::{MarketPanel, MarketView};
pub use view::{panel_text, row_text, table_selection, visible_rows};

#[cfg(test)]
mod tests;
