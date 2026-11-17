//! Owned hireling identity selection and outfits (existing capability250).
pub use oteryn_client_runtime::hireling::hireling::management as wire;
mod input;
mod protocol;
mod state;
mod view;
pub use input::select;
pub use protocol::{apply_result, outfits, request};
pub use state::{Intent, Panel};
pub use view::panel_text;
#[cfg(test)]
#[path = "tests/e2e_e12.rs"]
mod e2e_e12;
#[cfg(test)]
mod tests;
