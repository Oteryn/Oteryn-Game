//! Native projection of server-owned forge creature markers.
pub mod protocol;
mod view;
pub use view::label;

mod state;
pub use state::ForgeMarker;

#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
