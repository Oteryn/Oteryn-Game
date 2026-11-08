//! Native controls for Store.
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{apply_result, catalogue, claim, command, history, purchase, request};
pub use state::{StorePage, StorePanel};
pub use view::panel_text;
#[cfg(test)]
#[path = "tests/e2e_e10.rs"]
mod e2e_e10;
