//! Gem Atelier v2 (capability 483): domain-free gems, twelve ordered Gem Vessel slots and
//! Replace Vessel. The Wheel panel of mechanic 148 owns the screen and consumes this projection.
pub use oteryn_client_runtime::wheel_allocation::{WheelCommand, atelier};
mod protocol;
mod state;
mod view;
pub use protocol::{decode_reply, mutate, query, replace_vessel, reshape, set_lock};
pub use state::{Projection, ReplaceBlock, VesselBoard, project, replace_block};
pub use view::vessel_lines;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
