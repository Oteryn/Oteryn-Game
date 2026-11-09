//! Server-confirmed market offers and controls.
pub use oteryn_client_runtime::market::{CommandStatus, MarketCommand, MarketResponse, market};
#[path = "state/coins.rs"]
mod coins;
mod input;
pub use coins::{CoinPanel, coin_control, coin_result, coin_text};
mod protocol;
mod state;
mod view;
pub use input::{coin_open_key, edit_details, open_key};
pub use protocol::{Intent, apply_result, request};
pub use state::{MarketPanel, MarketView};
pub use view::{coin_panel_text, panel_text, row_text, visible_rows};

#[cfg(test)]
mod tests;
