//! Server-confirmed market offers and controls.
pub use oteryn_client_runtime::market::{CommandStatus, MarketCommand, MarketResponse, market};
mod input;
mod protocol;
mod state;
mod view;
pub use input::{edit_details, open_key};
pub use protocol::{Intent, apply_result, request};
pub use state::{MarketPanel, MarketView};
pub use view::{panel_text, row_text};

#[cfg(test)]
#[path = "tests/e2e_e05.rs"]
mod e2e_e05;
#[cfg(test)]
#[path = "tests/e2e_e06.rs"]
mod e2e_e06;
#[cfg(test)]
mod tests;
