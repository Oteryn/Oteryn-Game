//! Server-confirmed market offers and controls.
pub use oteryn_client_runtime::market::{CommandStatus, MarketCommand, MarketResponse, market};
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{Intent, apply_result, request};
pub use state::{MarketPanel, MarketView};
pub use view::{panel_text, row_text};

#[cfg(test)]
mod tests;
