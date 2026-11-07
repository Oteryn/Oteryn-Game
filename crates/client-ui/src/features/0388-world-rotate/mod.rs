//! Explicit Rotate shares the native world context menu and feedback projection.
mod protocol;
mod state;
pub use protocol::{apply_probe_result, probe_request, request};
pub use state::{InventoryProbe, MapBinding};
#[cfg(test)]
mod tests;
